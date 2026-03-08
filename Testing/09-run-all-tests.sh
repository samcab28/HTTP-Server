#!/bin/bash

# Script para ejecutar toda la suite de pruebas K6
# Proyecto Sistema Operativos - HTTP Server

echo "=========================================="
echo "  HTTP SERVER - SUITE DE PRUEBAS K6"
echo "=========================================="
echo ""

# Colores para output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Verificar que k6 está instalado
if ! command -v k6 &> /dev/null; then
    echo -e "${RED}Error: k6 no está instalado${NC}"
    echo "Instalar desde: https://k6.io/docs/getting-started/installation/"
    exit 1
fi

# Verificar que el servidor está corriendo
echo -e "${YELLOW}Verificando servidor...${NC}"
if ! curl -s http://localhost:8080/status > /dev/null; then
    echo -e "${RED}Error: Servidor no responde en http://localhost:8080${NC}"
    echo "Iniciar el servidor antes de ejecutar las pruebas"
    exit 1
fi
echo -e "${GREEN}✓ Servidor activo${NC}"
echo ""

# Crear directorio para resultados
RESULTS_DIR="results_$(date +%Y%m%d_%H%M%S)"
mkdir -p "$RESULTS_DIR"

echo "Resultados se guardarán en: $RESULTS_DIR"
echo ""

# Función para ejecutar prueba
run_test() {
    local test_file=$1
    local test_name=$2
    
    echo "=========================================="
    echo "  Ejecutando: $test_name"
    echo "=========================================="
    
    k6 run "$test_file" --out json="$RESULTS_DIR/${test_file%.js}.json" 2>&1 | tee "$RESULTS_DIR/${test_file%.js}.log"
    
    if [ ${PIPESTATUS[0]} -eq 0 ]; then
        echo -e "${GREEN}✓ $test_name completado${NC}"
    else
        echo -e "${RED}✗ $test_name falló${NC}"
    fi
    echo ""
    sleep 2
}

# Ejecutar pruebas en orden
run_test "01-unit-test.js" "Pruebas Unitarias"
run_test "02-race-conditions.js" "Pruebas de Concurrencia"
run_test "03-queue-overflow.js" "Pruebas de Cola"
run_test "04-io-intensive.js" "Pruebas IO Intensivas"
run_test "05-cpu-intensive.js" "Pruebas CPU Intensivas"
run_test "06-load-light.js" "Carga Ligera"
run_test "07-load-medium.js" "Carga Media"
run_test "08-load-heavy.js" "Carga Pesada"

# Resumen final
echo "=========================================="
echo "  RESUMEN DE PRUEBAS"
echo "=========================================="
echo ""
echo "Pruebas completadas:"
echo "  ✓ Unitarias (casos exitosos y fallidos)"
echo "  ✓ Concurrencia (50 VUs, race conditions)"
echo "  ✓ Cola (overflow con >2N trabajos)"
echo "  ✓ IO Intensivas (archivos ≥50MB)"
echo "  ✓ CPU Intensivas (operaciones pesadas)"
echo "  ✓ Carga Ligera (10 VUs)"
echo "  ✓ Carga Media (50 VUs)"
echo "  ✓ Carga Pesada (100 VUs)"
echo ""
echo "Resultados guardados en: $RESULTS_DIR"
echo ""
echo "Para ver logs individuales:"
echo "  cat $RESULTS_DIR/*.log"
echo ""
echo "=========================================="
echo -e "${GREEN}Suite de pruebas finalizada${NC}"
echo "=========================================="