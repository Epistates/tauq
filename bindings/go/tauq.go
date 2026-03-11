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

unsigned char* tauq_to_tbf(const char* input, size_t* out_len);
char* tauq_tbf_to_json(const unsigned char* data, size_t len);
char* tauq_tbf_to_tauq(const unsigned char* data, size_t len);
void tauq_free_buffer(unsigned char* ptr, size_t len);

size_t tauq_get_last_error(char* buffer, size_t size);

*/
import "C"
import (
	"encoding/json"
	"errors"
	"unsafe"
)

// getLastErrorMessage retrieves the detailed error message from the Rust core
func getLastErrorMessage() string {
	length := C.tauq_get_last_error(nil, 0)
	if length == 0 {
		return "unknown error"
	}

	buf := (*C.char)(C.malloc(C.size_t(length + 1)))
	defer C.free(unsafe.Pointer(buf))

	C.tauq_get_last_error(buf, length+1)
	return C.GoString(buf)
}

// Parse parses a Tauq string and returns the JSON string representation
func ParseToJSON(input string) (string, error) {
	cInput := C.CString(input)
	defer C.free(unsafe.Pointer(cInput))

	cResult := C.tauq_to_json(cInput)
	if cResult == nil {
		return "", errors.New(getLastErrorMessage())
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
		return "", errors.New(getLastErrorMessage())
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
		return "", errors.New(getLastErrorMessage())
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
		return "", errors.New(getLastErrorMessage())
	}
	defer C.tauq_free_string(cResult)

	return C.GoString(cResult), nil
}

// ToTBF converts a Tauq or JSON string to TBF bytes.
func ToTBF(input string) ([]byte, error) {
	cInput := C.CString(input)
	defer C.free(unsafe.Pointer(cInput))

	var outLen C.size_t
	cResult := C.tauq_to_tbf(cInput, &outLen)
	if cResult == nil {
		return nil, errors.New(getLastErrorMessage())
	}
	defer C.tauq_free_buffer(cResult, outLen)

	return C.GoBytes(unsafe.Pointer(cResult), C.int(outLen)), nil
}

// TBFToJSON converts TBF bytes to a JSON string.
func TBFToJSON(data []byte) (string, error) {
	if len(data) == 0 {
		return "", errors.New("empty data")
	}
	cData := (*C.uchar)(unsafe.Pointer(&data[0]))
	cLen := C.size_t(len(data))

	cResult := C.tauq_tbf_to_json(cData, cLen)
	if cResult == nil {
		return "", errors.New(getLastErrorMessage())
	}
	defer C.tauq_free_string(cResult)

	return C.GoString(cResult), nil
}

// TBFToTauq converts TBF bytes to a Tauq string.
func TBFToTauq(data []byte) (string, error) {
	if len(data) == 0 {
		return "", errors.New("empty data")
	}
	cData := (*C.uchar)(unsafe.Pointer(&data[0]))
	cLen := C.size_t(len(data))

	cResult := C.tauq_tbf_to_tauq(cData, cLen)
	if cResult == nil {
		return "", errors.New(getLastErrorMessage())
	}
	defer C.tauq_free_string(cResult)

	return C.GoString(cResult), nil
}

// Unmarshal parses Tauq-encoded data and stores the result in the value pointed to by v.
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
