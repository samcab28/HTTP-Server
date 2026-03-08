// Pruebas Unitarias - Casos exitosos y fallidos por ruta
import http from 'k6/http';
import { check } from 'k6';

export const options = {
  vus: 1,
  iterations: 1,
  thresholds: {
    checks: ['rate>0.95'],
  },
};

const BASE = 'http://localhost:8080';

export default function () {
  let res;
  
  // ============ REVERSE ============
  res = http.get(`${BASE}/reverse?text=hello`);
  check(res, { 'reverse ok': r => r.status === 200 && r.json().reversed === 'olleh' });


  res = http.get(`${BASE}/reverse`);
  check(res, { 'reverse missing param': r => r.status === 400 });
  
  // ============ FIBONACCI ============
  res = http.get(`${BASE}/fibonacci?num=10`);
  check(res, { 'fibonacci ok': r => r.status === 200 && r.json().result === 55 });
  
  res = http.get(`${BASE}/fibonacci?num=100`);
  check(res, { 'fibonacci out of range': r => r.status === 400 });
  
  res = http.get(`${BASE}/fibonacci?num=abc`);
  check(res, { 'fibonacci invalid param': r => r.status === 400 });
  
  // ============ RANDOM ============
  res = http.get(`${BASE}/random?count=10&min=1&max=100`);
  check(res, { 'random ok': r => r.status === 200 && r.json().numbers.length === 10 });
  
  res = http.get(`${BASE}/random?count=20000`);
  check(res, { 'random exceed limit': r => r.status === 400 });
  
  // ============ ISPRIME ============
  res = http.get(`${BASE}/isprime?n=17&algo=division`);
  check(res, { 'isprime ok': r => r.status === 200 && r.json().is_prime === true });
  
  res = http.get(`${BASE}/isprime?n=abc&algo=division`);
  check(res, { 'isprime invalid n': r => r.status === 400 });
  
  // ============ FACTOR ============
  res = http.get(`${BASE}/factor?n=360`);
  check(res, { 'factor ok': r => r.status === 200 && Array.isArray(r.json().factors) });
  
  // ============ WORDCOUNT ============
  res = http.get(`${BASE}/wordcount?name=grep.txt`);
  check(res, { 'wordcount ok': r => r.status === 200 });
  
  res = http.get(`${BASE}/wordcount?name=noexiste.txt`);
  check(res, { 'wordcount file not found': r => r.status === 404 });
  
  // ============ GREP ============
  res = http.get(`${BASE}/grep?name=grep.txt&pattern=He`);
  check(res, { 'grep ok': r => r.status === 200 });
  
  res = http.get(`${BASE}/grep?name=grep.txt`);
  check(res, { 'grep missing pattern': r => r.status === 400 });
  
  // ============ CREATEFILE ============
  res = http.get(`${BASE}/createfile?name=../etc/passwd&content=hack`);
  check(res, { 'createfile path traversal blocked': r => r.status === 400 });
  
  // ============ 404 ============
  res = http.get(`${BASE}/notfound`);
  check(res, { '404 route not found': r => r.status === 404 });
  
  console.log('✓ Unit tests completed');
}