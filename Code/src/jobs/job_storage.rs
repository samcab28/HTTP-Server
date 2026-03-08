use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use super::job_types::{now_ms, Job, JobPriority, JobStatus};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CancelOutcome { Canceled, NotFound, NotCancelable }

#[derive(Default)]
pub struct InMemoryStorage {
    jobs: HashMap<String, Job>,
}

static STORAGE: OnceLock<Mutex<InMemoryStorage>> = OnceLock::new();

pub fn storage() -> &'static Mutex<InMemoryStorage> {
    STORAGE.get_or_init(|| {
        let mut st = InMemoryStorage::default();
        // Try to load persisted jobs; ignore errors silently
        let _ = st.load_from_disk();
        Mutex::new(st)
    })
}

impl InMemoryStorage {
    pub fn insert(&mut self, job: Job) {
        let id = job.id.clone();
        self.jobs.insert(id.clone(), job);
        let _ = self.persist_job_by_id(&id);
    }
    pub fn get(&self, id: &str) -> Option<Job> { self.jobs.get(id).cloned() }
    pub fn update(&mut self, job: Job) {
        let id = job.id.clone();
        self.jobs.insert(id.clone(), job);
        let _ = self.persist_job_by_id(&id);
    }
    pub fn list(&self) -> Vec<Job> { self.jobs.values().cloned().collect() }
    pub fn cancel(&mut self, id: &str) -> CancelOutcome {
        if let Some(j) = self.jobs.get_mut(id) {
            match j.status {
                JobStatus::Queued | JobStatus::Running => {
                    j.status = JobStatus::Canceled;
                    j.finished_at = Some(now_ms());
                    let _ = self.persist_job_by_id(id);
                    CancelOutcome::Canceled
                }
                _ => CancelOutcome::NotCancelable,
            }
        } else { CancelOutcome::NotFound }
    }

    fn jobs_root() -> PathBuf {
        let base = crate::handlers::data_dir().to_string();
        let mut p = PathBuf::from(base);
        p.push("jobs");
        p
    }

    fn persist_job_by_id(&self, id: &str) -> std::io::Result<()> {
        if let Some(job) = self.jobs.get(id) {
            Self::persist_job(job)
        } else { Ok(()) }
    }

    fn persist_job(job: &Job) -> std::io::Result<()> {
        let root = Self::jobs_root();
        fs::create_dir_all(&root)?;
        let mut dir = root;
        dir.push(&job.id);
        fs::create_dir_all(&dir)?;

        // Write meta.txt
        let mut meta = String::new();
        meta.push_str(&format!("id:{}\n", job.id));
        meta.push_str(&format!("route:{}\n", job.route));
        meta.push_str(&format!("status:{}\n", match job.status { JobStatus::Queued=>"Queued", JobStatus::Running=>"Running", JobStatus::Done=>"Done", JobStatus::Failed=>"Failed", JobStatus::Canceled=>"Canceled" }));
        meta.push_str(&format!("priority:{}\n", match job.priority { JobPriority::High=>"High", JobPriority::Normal=>"Normal", JobPriority::Low=>"Low" }));
        meta.push_str(&format!("submitted_at:{}\n", job.submitted_at));
        meta.push_str(&format!("started_at:{}\n", job.started_at.map(|v| v.to_string()).unwrap_or_else(|| "-".into())));
        meta.push_str(&format!("finished_at:{}\n", job.finished_at.map(|v| v.to_string()).unwrap_or_else(|| "-".into())));
        meta.push_str(&format!("result_status:{}\n", job.result_status.map(|v| v.to_string()).unwrap_or_else(|| "-".into())));
        meta.push_str(&format!("error:{}\n", job.error.clone().unwrap_or_else(|| "-".into())));
        let params_enc = encode_params(&job.params);
        meta.push_str(&format!("params:{}\n", params_enc));

        let mut meta_path = dir.clone();
        meta_path.push("meta.txt");
        fs::write(meta_path, meta.as_bytes())?;

        // Write resp.bin if present
        let mut resp_path = dir.clone();
        resp_path.push("resp.bin");
        if let Some(bytes) = &job.result_raw {
            fs::write(resp_path, bytes)?;
        } else {
            // If previously existed, ignore errors when removing
            let _ = fs::remove_file(resp_path);
        }
        Ok(())
    }

    pub fn load_from_disk(&mut self) -> std::io::Result<()> {
        let root = Self::jobs_root();
        if !root.exists() { return Ok(()); }
        for entry in fs::read_dir(&root)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() { continue; }
            let id = entry.file_name().to_string_lossy().to_string();
            let mut meta_path = entry.path();
            meta_path.push("meta.txt");
            if !meta_path.exists() { continue; }
            let mut content = String::new();
            fs::File::open(&meta_path)?.read_to_string(&mut content)?;
            if let Some(job) = parse_meta(&id, &content) {
                // resp.bin (optional)
                let mut resp_path = entry.path();
                resp_path.push("resp.bin");
                if resp_path.exists() {
                    if let Ok(mut f) = fs::File::open(&resp_path) {
                        let mut buf = Vec::new();
                        let _ = f.read_to_end(&mut buf);
                        let mut job2 = job.clone();
                        job2.result_raw = Some(buf);
                        self.jobs.insert(id.clone(), job2);
                        continue;
                    }
                }
                self.jobs.insert(id.clone(), job);
            }
        }
        Ok(())
    }
}

fn encode_params(params: &HashMap<String, String>) -> String {
    // very small percent-like encoding to keep parse simple
    fn enc(s: &str) -> String {
        s.replace('%', "%25").replace('=', "%3D").replace('&', "%26")
    }
    let mut pairs: Vec<String> = Vec::new();
    for (k, v) in params.iter() {
        pairs.push(format!("{}={}", enc(k), enc(v)));
    }
    pairs.join("&")
}

fn decode_params(s: &str) -> HashMap<String, String> {
    fn dec(s: &str) -> String {
        // reverse order to avoid double-decoding
        s.replace("%26", "&").replace("%3D", "=").replace("%25", "%")
    }
    let mut out = HashMap::new();
    for part in s.split('&') {
        if part.is_empty() { continue; }
        let (k, v) = match part.split_once('=') { Some((k,v)) => (k, v), None => (part, "") };
        out.insert(dec(k), dec(v));
    }
    out
}

fn parse_meta(id: &str, meta: &str) -> Option<Job> {
    let mut route = String::new();
    let mut status = JobStatus::Queued;
    let mut priority = JobPriority::Normal;
    let mut submitted_at: u128 = 0;
    let mut started_at: Option<u128> = None;
    let mut finished_at: Option<u128> = None;
    let mut result_status: Option<u16> = None;
    let mut error: Option<String> = None;
    let mut params: HashMap<String, String> = HashMap::new();
    for line in meta.lines() {
        if let Some(v) = line.strip_prefix("route:") { route = v.to_string(); }
        else if let Some(v) = line.strip_prefix("status:") {
            status = match v.trim() {
                "Queued"=>JobStatus::Queued, "Running"=>JobStatus::Running, "Done"=>JobStatus::Done,
                "Failed"=>JobStatus::Failed, "Canceled"=>JobStatus::Canceled, _=>JobStatus::Queued
            };
        }
        else if let Some(v) = line.strip_prefix("priority:") {
            priority = match v.trim() { "High"=>JobPriority::High, "Low"=>JobPriority::Low, _=>JobPriority::Normal };
        }
        else if let Some(v) = line.strip_prefix("submitted_at:") { if let Ok(n) = v.trim().parse() { submitted_at = n; } }
        else if let Some(v) = line.strip_prefix("started_at:") {
            let t = v.trim(); if t != "-" { if let Ok(n) = t.parse() { started_at = Some(n); } }
        }
        else if let Some(v) = line.strip_prefix("finished_at:") {
            let t = v.trim(); if t != "-" { if let Ok(n) = t.parse() { finished_at = Some(n); } }
        }
        else if let Some(v) = line.strip_prefix("result_status:") {
            let t = v.trim(); if t != "-" { if let Ok(n) = t.parse() { result_status = Some(n); } }
        }
        else if let Some(v) = line.strip_prefix("error:") {
            let t = v.trim(); if t != "-" { error = Some(t.to_string()); }
        }
        else if let Some(v) = line.strip_prefix("params:") { params = decode_params(v.trim()); }
    }
    if route.is_empty() { return None; }
    let mut job = Job::new(id.to_string(), route, params);
    job.status = status;
    job.priority = priority;
    job.submitted_at = submitted_at;
    job.started_at = started_at;
    job.finished_at = finished_at;
    job.result_status = result_status;
    job.error = error;
    Some(job)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_params_roundtrip() {
        let mut m = HashMap::new();
        m.insert("a".into(), "1&2=3%".into());
        m.insert("b=b".into(), "x&y".into());
        let enc = encode_params(&m);
        let dec = decode_params(&enc);
        assert_eq!(m, dec);
    }
}
