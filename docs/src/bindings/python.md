# Python Bindings

## Installation

```bash
pip install tauq
```

## Usage

```python
import tauq

# 1. Parse Tauq
data = tauq.loads("""
!def User id name
1 Alice
2 Bob
""")
print(data)  # Output: [{'id': 1, 'name': 'Alice'}, {'id': 2, 'name': 'Bob'}]

# 2. Format to Tauq
obj = [{"id": 1, "name": "Alice"}]
tqn = tauq.dumps(obj)
print(tqn)

# 3. Execute Query
# (Supports shell scripts via !run, !emit, !pipe)
result = tauq.exec_tauqq("!emit echo 'status ok'")

# 4. Minify
minified = tauq.minify("!def A x; 1; 2; 3")
```