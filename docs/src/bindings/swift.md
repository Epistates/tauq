# Swift Bindings

## Installation

Add the package via Swift Package Manager (SPM):

```swift
.package(url: "https://github.com/epistates/tauq.git", from: "0.1.0")
```

## Usage

```swift
import Tauq

// Note: !def implies !use, so data rows immediately follow
let input = """
!def User id name
1 Alice
2 Bob
"""

// 1. Parse to JSON String
let json = try Tauq.toJSON(input)
print(json)
// Use Codable to parse 'json' string

// 2. Format
let tqn = try Tauq.toTauq("[{\"id\": 1, \"name\": \"Alice\"}]")
print(tqn)

// 3. Exec Query
let res = try Tauq.execQuery("!emit echo '1 Alice'", safeMode: true)

// 4. Minify
let min = try Tauq.minify("!def T x; 1; 2; 3")
print(min)
```