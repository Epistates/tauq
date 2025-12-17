#!/bin/bash
# Simple curl test to see raw response

echo "=========================================="
echo "TEST: Simple question with JSON data"
echo "=========================================="

curl http://localhost:1234/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-oss-120b",
    "messages": [
        {
            "role": "system",
            "content": "You are a data analysis assistant. Answer concisely with just the final answer."
        },
        {
            "role": "user",
            "content": "Data: {\"employees\": [{\"id\": 1, \"name\": \"Alice\"}, {\"id\": 2, \"name\": \"Bob\"}, {\"id\": 3, \"name\": \"Carol\"}]}\n\nQuestion: How many employees are there?\n\nAnswer with ONLY the number."
        }
    ],
    "temperature": 0.0,
    "max_tokens": -1,
    "stream": false
}' 2>/dev/null | python3 -m json.tool

echo ""
echo "=========================================="
echo "TEST: Same question with Tauq format"
echo "=========================================="

TAUQ_SYSTEM='You are analyzing data in Tauq format.

Tauq format uses schemas:
- "!def TypeName field1 field2" defines a schema
- Lines after !def are data rows
- Count the data lines to count records

Example:
!def User id name
1 Alice
2 Bob

This is 2 users (2 data lines).'

TAUQ_DATA='employees [
  !use Employee
  1 Alice
  2 Bob
  3 Carol
]'

curl http://localhost:1234/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d "{
    \"model\": \"gpt-oss-120b\",
    \"messages\": [
        {
            \"role\": \"system\",
            \"content\": \"$TAUQ_SYSTEM\"
        },
        {
            \"role\": \"user\",
            \"content\": \"Data: $TAUQ_DATA\\n\\nQuestion: How many employees are there?\\n\\nAnswer with ONLY the number.\"
        }
    ],
    \"temperature\": 0.0,
    \"max_tokens\": -1,
    \"stream\": false
}" 2>/dev/null | python3 -m json.tool
