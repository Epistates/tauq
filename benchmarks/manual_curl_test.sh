#!/bin/bash
# Manual curl test with Tauq format cheat sheet

# Generate test data first
cat > /tmp/test_data_tauq.txt <<'EOF'
!def Employee id name email department salary yearsExperience active
---
employees [
  !use Employee
  1 Employee1 employee1@company.com Engineering 45000 1 false
  2 Employee2 employee2@company.com Sales 46000 2 true
  3 Employee3 employee3@company.com Marketing 47000 3 true
  4 Employee4 employee4@company.com HR 48000 4 true
  5 Employee5 employee5@company.com Operations 49000 5 true
]
EOF

cat > /tmp/test_data_json.txt <<'EOF'
{
  "employees": [
    {"id": 1, "name": "Employee1", "email": "employee1@company.com", "department": "Engineering", "salary": 45000, "yearsExperience": 1, "active": false},
    {"id": 2, "name": "Employee2", "email": "employee2@company.com", "department": "Sales", "salary": 46000, "yearsExperience": 2, "active": true},
    {"id": 3, "name": "Employee3", "email": "employee3@company.com", "department": "Marketing", "salary": 47000, "yearsExperience": 3, "active": true},
    {"id": 4, "name": "Employee4", "email": "employee4@company.com", "department": "HR", "salary": 48000, "yearsExperience": 4, "active": true},
    {"id": 5, "name": "Employee5", "email": "employee5@company.com", "department": "Operations", "salary": 49000, "yearsExperience": 5, "active": true}
  ]
}
EOF

TAUQ_DATA=$(cat /tmp/test_data_tauq.txt)
JSON_DATA=$(cat /tmp/test_data_json.txt)

TAUQ_CHEATSHEET="You are analyzing data in Tauq format - a token-efficient notation.

Tauq Syntax Rules:
- !def TypeName field1 field2 ... = defines a schema with field names
- After !def, each line is ONE data row with values in field order
- !use TypeName = activates a previously defined schema
- Data rows can appear directly after !def OR inside arrays after !use

Counting Records:
- Count the number of data lines (not including !def or !use lines)
- Each data line = 1 complete record

Example:
!def User id name
1 Alice
2 Bob
3 Carol

This is 3 users (3 data lines after !def).

Example with array:
users [
  !use User
  1 Alice
  2 Bob
]

This is also 2 users (2 data lines after !use).

Your task: Read the data and answer questions accurately."

echo "=========================================="
echo "TEST 1: JSON FORMAT"
echo "=========================================="
curl http://localhost:1234/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d "{
    \"model\": \"gpt-oss-120b\",
    \"messages\": [
        {
            \"role\": \"system\",
            \"content\": \"You are a data analysis assistant. Answer concisely.\"
        },
        {
            \"role\": \"user\",
            \"content\": \"Data:\\n${JSON_DATA}\\n\\nQuestion: How many employees are there?\\n\\nProvide ONLY the number.\"
        }
    ],
    \"temperature\": 0.0,
    \"max_tokens\": -1,
    \"stream\": false
}" 2>/dev/null | jq '.choices[0].message | {content, reasoning, finish_reason: .finish_reason}'

echo ""
echo "=========================================="
echo "TEST 2: TAUQ FORMAT (with cheat sheet)"
echo "=========================================="

# Need to properly escape the cheatsheet and data for JSON
ESCAPED_CHEATSHEET=$(echo "$TAUQ_CHEATSHEET" | jq -Rs .)
ESCAPED_TAUQ_DATA=$(echo "$TAUQ_DATA" | jq -Rs .)

curl http://localhost:1234/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d "{
    \"model\": \"gpt-oss-120b\",
    \"messages\": [
        {
            \"role\": \"system\",
            \"content\": $ESCAPED_CHEATSHEET
        },
        {
            \"role\": \"user\",
            \"content\": \"Data:\\n${TAUQ_DATA}\\n\\nQuestion: How many employees are there?\\n\\nProvide ONLY the number.\"
        }
    ],
    \"temperature\": 0.0,
    \"max_tokens\": -1,
    \"stream\": false
}" 2>/dev/null | jq '.choices[0].message | {content, reasoning, finish_reason: .finish_reason}'

echo ""
echo "=========================================="
echo "Done!"
echo "=========================================="
