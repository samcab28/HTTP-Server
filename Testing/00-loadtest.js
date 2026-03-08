import http from 'k6/http';
import { check, sleep } from 'k6';

// Configuración del test
export const options = {
  vus: 50,           // 50 usuarios virtuales simultáneos
  duration: '10s',   // durante 10 segundos
};

export default function () {
  const res = http.get('http://localhost:8080/wordcount?name=grep.txt');
  
  // Validaciones básicas
  check(res, {
    'status es 200': (r) => r.status === 200,
    'respuesta no vacía': (r) => r.body.length > 0,
  });

  sleep(0.1); // pequeña pausa entre solicitudes
}

