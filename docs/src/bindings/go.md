# Go Bindings

## Installation

```bash
go get github.com/epistates/tauq
```

## Usage

The Go bindings provide an idiomatic `Marshal`/`Unmarshal` interface similar to `encoding/json`.

```go
package main

import (
	"fmt"
	"github.com/epistates/tauq"
)

type User struct {
	ID   int    `json:"id"`
	Name string `json:"name"`
}

func main() {
	// 1. Unmarshal (Parse)
	// Note: !def implies !use, so data rows immediately follow
	input := `
!def User id name
1 Alice
2 Bob
`
	var users []User
	err := tauq.Unmarshal([]byte(input), &users)
	if err != nil {
		panic(err)
	}
	fmt.Printf("%+v\n", users)

	// 2. Marshal (Format)
	bytes, _ := tauq.Marshal(users)
	fmt.Println(string(bytes))

	// 3. Exec Query
	tauq.Exec([]byte("!emit echo '1 Alice'"), true, &users) // safeMode=true

	// 4. Minify
	min, _ := tauq.Minify("!def T x; 1; 2")
	fmt.Println(min)
}
```