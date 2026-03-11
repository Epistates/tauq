import Foundation
import CTauq

/// Tauq (τq) Bindings for Swift
public class Tauq {
    public enum TauqError: Error, LocalizedError {
        case parseError(String)
        case formatError(String)
        case utf8Error
        
        public var errorDescription: String? {
            switch self {
            case .parseError(let msg): return "Tauq Parse Error: \(msg)"
            case .formatError(let msg): return "Tauq Format Error: \(msg)"
            case .utf8Error: return "Invalid UTF-8 sequence"
            }
        }
    }
    
    private static func getLastErrorMessage() -> String {
        let len = tauq_get_last_error(nil, 0)
        if len == 0 { return "Unknown error" }
        
        let buffer = UnsafeMutablePointer<Int8>.allocate(capacity: len + 1)
        defer { buffer.deallocate() }
        
        tauq_get_last_error(buffer, len + 1)
        return String(cString: buffer)
    }
    
    /// Parse Tauq source to JSON string.
    public static func toJSON(_ input: String) throws -> String {
        return try input.withCString { cString in
            guard let result = tauq_to_json(cString) else {
                throw TauqError.parseError(getLastErrorMessage())
            }
            defer { tauq_free_string(result) }
            
            guard let str = String(cString: result, encoding: .utf8) else {
                throw TauqError.utf8Error
            }
            return str
        }
    }

    /// Execute Tauq Query (TQQ) source to JSON string.
    public static func execQuery(_ input: String, safeMode: Bool = false) throws -> String {
        return try input.withCString { cString in
            guard let result = tauq_exec_query(cString, safeMode) else {
                throw TauqError.parseError(getLastErrorMessage())
            }
            defer { tauq_free_string(result) }
            
            guard let str = String(cString: result, encoding: .utf8) else {
                throw TauqError.utf8Error
            }
            return str
        }
    }
    
    /// Minify Tauq source to single-line Tauq string.
    public static func minify(_ input: String) throws -> String {
        return try input.withCString { cString in
            guard let result = tauq_minify(cString) else {
                throw TauqError.parseError(getLastErrorMessage())
            }
            defer { tauq_free_string(result) }
            
            guard let str = String(cString: result, encoding: .utf8) else {
                throw TauqError.utf8Error
            }
            return str
        }
    }
    
    /// Format JSON string to Tauq.
    public static func toTauq(_ input: String) throws -> String {
        return try input.withCString { cString in
            guard let result = json_to_tauq_c(cString) else {
                throw TauqError.formatError(getLastErrorMessage())
            }
            defer { tauq_free_string(result) }
            
            guard let str = String(cString: result, encoding: .utf8) else {
                throw TauqError.utf8Error
            }
            return str
        }
    }
    
    /// Convert Tauq or JSON string to TBF bytes.
    public static func toTBF(_ input: String) throws -> Data {
        return try input.withCString { cString in
            var outLen: Int = 0
            guard let result = tauq_to_tbf(cString, &outLen) else {
                throw TauqError.formatError(getLastErrorMessage())
            }
            defer { tauq_free_buffer(result, outLen) }
            
            return Data(bytes: result, count: outLen)
        }
    }

    /// Convert TBF bytes to JSON string.
    public static func tbfToJSON(_ data: Data) throws -> String {
        return try data.withUnsafeBytes { buffer in
            guard let ptr = buffer.bindMemory(to: UInt8.self).baseAddress else {
                throw TauqError.utf8Error
            }
            guard let result = tauq_tbf_to_json(ptr, data.count) else {
                throw TauqError.parseError(getLastErrorMessage())
            }
            defer { tauq_free_string(result) }
            
            guard let str = String(cString: result, encoding: .utf8) else {
                throw TauqError.utf8Error
            }
            return str
        }
    }

    /// Convert TBF bytes to Tauq string.
    public static func tbfToTauq(_ data: Data) throws -> String {
        return try data.withUnsafeBytes { buffer in
            guard let ptr = buffer.bindMemory(to: UInt8.self).baseAddress else {
                throw TauqError.utf8Error
            }
            guard let result = tauq_tbf_to_tauq(ptr, data.count) else {
                throw TauqError.parseError(getLastErrorMessage())
            }
            defer { tauq_free_string(result) }
            
            guard let str = String(cString: result, encoding: .utf8) else {
                throw TauqError.utf8Error
            }
            return str
        }
    }
}
