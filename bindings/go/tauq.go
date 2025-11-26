package tauq

/*
#cgo LDFLAGS: -L../../target/release -ltauq
#include <stdlib.h>

// Forward declarations of C functions from Rust
char* tauq_to_json(const char* input);
char* tauq_exec_query(const char* input, bool safe_mode);
char* tauq_minify(const char* input);
char* json_to_tauq_c(const char* input);
void tauq_free_string(char* s);

*/
import "C"
import (
	"encoding/json"
	"errors"
	"unsafe"
)

// Parse parses a Tauq string and returns the JSON string representation
// (Intermediate step before unmarshaling to Go struct)
func ParseToJSON(input string) (string, error) {
	cInput := C.CString(input)
	defer C.free(unsafe.Pointer(cInput))

	cResult := C.tauq_to_json(cInput)
	if cResult == nil {
		return "", errors.New("failed to parse tauq")
	}
	defer C.tauq_free_string(cResult)

	return C.GoString(cResult), nil
}

// ExecQueryToJSON executes a Tauq Query and returns JSON string
func ExecQueryToJSON(input string, safeMode bool) (string, error) {
	cInput := C.CString(input)
	defer C.free(unsafe.Pointer(cInput))

	cResult := C.tauq_exec_query(cInput, C.bool(safeMode))
	if cResult == nil {
		return "", errors.New("failed to execute tauq query")
	}
	defer C.tauq_free_string(cResult)

	return C.GoString(cResult), nil
}

// Minify compresses Tauq source to a single line
func Minify(input string) (string, error) {
	cInput := C.CString(input)
	defer C.free(unsafe.Pointer(cInput))

	cResult := C.tauq_minify(cInput)
	if cResult == nil {
		return "", errors.New("failed to minify tauq")
	}
	defer C.tauq_free_string(cResult)

	return C.GoString(cResult), nil
}

// FormatJSON converts a JSON string to Tauq format
func FormatJSON(inputJSON string) (string, error) {
	cInput := C.CString(inputJSON)
	defer C.free(unsafe.Pointer(cInput))

	cResult := C.json_to_tauq_c(cInput)
	if cResult == nil {
		return "", errors.New("failed to format json")
	}
	defer C.tauq_free_string(cResult)

	return C.GoString(cResult), nil
}

// Unmarshal parses Tauq-encoded data and stores the result in the value pointed to by v.
// It behaves like json.Unmarshal but for Tauq.
func Unmarshal(data []byte, v interface{}) error {
	jsonStr, err := ParseToJSON(string(data))
	if err != nil {
		return err
	}
	return json.Unmarshal([]byte(jsonStr), v)
}

// Exec unmarshals the result of a Tauq Query into v.
func Exec(data []byte, safeMode bool, v interface{}) error {
	jsonStr, err := ExecQueryToJSON(string(data), safeMode)
	if err != nil {
		return err
	}
	return json.Unmarshal([]byte(jsonStr), v)
}

// Marshal returns the Tauq encoding of v.
// It behaves like json.Marshal but returns Tauq.
func Marshal(v interface{}) ([]byte, error) {
	jsonData, err := json.Marshal(v)
	if err != nil {
		return nil, err
	}
	
	tauqStr, err := FormatJSON(string(jsonData))
	if err != nil {
		return nil, err
	}
	
	return []byte(tauqStr), nil
}