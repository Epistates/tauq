# Tauq Query (TQQ)

**Tauq Query (`.tqq`) is a pre-processor.** 

It does not query the data *after* it's loaded (like SQL). Instead, it executes *before* the Tauq parser sees the data. It generates valid Tauq Notation (`.tqn`) which is then parsed into your final JSON/object output.

Think of it like a PHP file generating HTML, but for data.

## The Pipeline Model

TQQ files are processed from top to bottom. Directives (starting with `!`) perform actions like setting variables, running commands, or piping data.

### 1. Variables and Environment
You can inject dynamic values into your data stream.

```tqn
!set VERSION "1.0.0"
!env HOME

app_version "v1.0.0"
user_home "/Users/nick"
```

### 2. Generating Data (`!emit`)
Use `!emit` to run a shell command and insert its output directly into the stream.

```tqn
!def File name size

!use File
!emit ls -sh *.rs | awk 
'{print "\" " $2 " \" " $1}'
```
*Note: The command output must be valid Tauq syntax (or close enough to be parsed).*

### 3. The `!pipe` Directive
The `!pipe` directive is special. It captures **everything that follows it** and sends it as input (stdin) to a shell command. The output of that command then replaces the original content.

**Example: Sorting data**
```tqn
!def Score player points
!use Score

!pipe sort -k 2 -nr  # Sort by 2nd column (points), numeric, reverse

"Alice" 50
"Bob"   20
"Carol" 95
```

**How it works:**
1. TQQ reads `!pipe sort ...`.
2. It consumes the rest of the file:
   ```text
   "Alice" 50
   "Bob"   20
   "Carol" 95
   ```
3. It sends this text to `sort`.
4. `sort` returns:
   ```text
   "Carol" 95
   "Alice" 50
   "Bob"   20
   ```
5. This sorted text replaces the original lines in the final output.

### 4. Reading External Data
You can pull in raw JSON or other Tauq files.

```tqn
!json "data.json"   # Reads JSON, converts to Tauq, inserts here
!import "common.tqn" # Inserts content of common.tqn here
```

### 5. Scripting (`!run`)
For complex logic, embed a script (Python, Ruby, Node, etc.).

```tqn
!run python3 {
    import random
    print('!def Random val')
    for _ in range(3):
        print(random.randint(1, 100))
}
```

**Note:** The script's stdout is inserted into the stream. Since `!def` implies `!use`, the generated data is immediately parsed as `Random` rows.