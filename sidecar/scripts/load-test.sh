#!/bin/bash

while true; do
  curl -s -X POST http://localhost:8080/order \
    -H "Content-Type: application/json" \
    -d "{\"product_id\": \"PROD$(($RANDOM % 10))\", \"quantity\": $(($RANDOM % 5 + 1))}" > /dev/null
  sleep 0.5
done