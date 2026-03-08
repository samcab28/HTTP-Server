#!/bin/bash

# Script para crear archivos de prueba necesarios
# Proyecto Sistema Operativos - HTTP Server

echo "=========================================="
echo "  CREANDO ARCHIVOS DE PRUEBA"
echo "=========================================="
echo ""

# Directorio de datos (ajustar según tu estructura)
DATA_DIR="../Code/data"

# Crear directorio si no existe
if [ ! -d "$DATA_DIR" ]; then
    echo "Creando directorio: $DATA_DIR"
    mkdir -p "$DATA_DIR"
fi

cd "$DATA_DIR" || exit 1

echo "Directorio de trabajo: $(pwd)"
echo ""

# ========================================
# Archivo pequeño para pruebas básicas
# ========================================
if [ ! -f "test.txt" ]; then
    echo "→ Creando test.txt (pequeño)..."
    echo "Hello World Test Line" > test.txt
    for i in {1..100}; do
        echo "Line $i: Lorem ipsum dolor sit amet consectetur adipiscing elit" >> test.txt
    done
    echo "  ✓ test.txt creado ($(du -h test.txt | cut -f1))"
else
    echo "  ✓ test.txt ya existe"
fi

# ========================================
# Archivo para grep
# ========================================
if [ ! -f "grep.txt" ]; then
    echo "→ Creando grep.txt..."
    {
        echo "Hello World"
        echo "Testing grep functionality"
        echo "Hello again"
        echo "Some random text here"
        echo "HELLO in uppercase"
        echo "hello in lowercase"
        for i in {1..1000}; do
            echo "Line $i with some test data"
        done
    } > grep.txt
    echo "  ✓ grep.txt creado ($(du -h grep.txt | cut -f1))"
else
    echo "  ✓ grep.txt ya existe"
fi

# ========================================
# Archivo 50MB para IO intensive
# ========================================
echo "Generando archivo de 50MB... (toma 1-2 minutos)"
{
  for i in {1..500000}; do
    echo "$RANDOM $(date +%s%N) Line $i with random data content text padding extra"
  done
} > data-sort-50mb.txt

# ========================================
# Archivo 100MB (opcional para más estrés)
# ========================================
read -p "¿Crear archivo de 100MB también? (s/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Ss]$ ]]; then
    if [ ! -f "data-sort-100mb.txt" ]; then
        echo "→ Creando data-sort-100mb.txt (100MB)..."
        echo "  (Esto puede tomar 2-3 minutos...)"
        
        for i in {1..1000000}; do
            echo "$RANDOM $(date +%s%N) Line $i with random data $(openssl rand -hex 20)"
        done > data-sort-100mb.txt
        
        SIZE=$(du -h data-sort-100mb.txt | cut -f1)
        echo "  ✓ data-sort-100mb.txt creado ($SIZE)"
    else
        echo "  ✓ data-sort-100mb.txt ya existe"
    fi
fi

# ========================================
# Resumen
# ========================================
echo ""
echo "=========================================="
echo "  RESUMEN DE ARCHIVOS CREADOS"
echo "=========================================="
ls -lh test.txt grep.txt data-sort-*.txt 2>/dev/null | awk '{print $9, $5}'
echo ""
echo "Archivos listos en: $(pwd)"
echo ""
echo "Ahora puedes ejecutar:"
echo "  cd ../Testing"
echo "  k6 run 04-io-intensive.js"
echo "=========================================="