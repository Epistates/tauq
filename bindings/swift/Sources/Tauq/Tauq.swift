import Foundation
import CTauq

/// Tauq (τq) Bindings for Swift
public class Tauq {
    public enum TauqError: Error {
        case parseError
        case formatError
        case utf8Error
    }
    
    /// Parse Tauq source to JSON string.
    /// - Parameter input: Tauq source string
    /// - Returns: JSON string
    /// - Throws: TauqError
    public static func toJSON(_ input: String) throws -> String {
        return try input.withCString { cString in
            guard let result = tauq_to_json(cString) else {
                throw TauqError.parseError
            }
            defer { tauq_free_string(result) }
            
            guard let str = String(cString: result, encoding: .utf8) else {
                throw TauqError.utf8Error
            }
            return str
        }
    }

    /// Execute Tauq Query (TQQ) source to JSON string.
    /// - Parameter input: TQQ source string
    /// - Parameter safeMode: If true, disables unsafe directives
    /// - Returns: JSON string
    /// - Throws: TauqError
    public static func execQuery(_ input: String, safeMode: Bool = false) throws -> String {
        return try input.withCString { cString in
            guard let result = tauq_exec_query(cString, safeMode) else {
                throw TauqError.parseError
            }
            defer { tauq_free_string(result) }
            
            guard let str = String(cString: result, encoding: .utf8) else {
                throw TauqError.utf8Error
            }
            return str
        }
    }
    
    /// Minify Tauq source to single-line Tauq string.
    /// - Parameter input: Tauq source string
    /// - Returns: Minified Tauq string
    /// - Throws: TauqError
    public static func minify(_ input: String) throws -> String {
        return try input.withCString { cString in
            guard let result = tauq_minify(cString) else {
                throw TauqError.parseError
            }
            defer { tauq_free_string(result) }
            
            guard let str = String(cString: result, encoding: .utf8) else {
                throw TauqError.utf8Error
            }
            return str
        }
    }
    
    /// Format JSON string to Tauq.
    /// - Parameter input: JSON string
    /// - Returns: Tauq string
    /// - Throws: TauqError
    public static func toTauq(_ input: String) throws -> String {
        return try input.withCString { cString in
            guard let result = json_to_tauq_c(cString) else {
                throw TauqError.formatError
            }
            defer { tauq_free_string(result) }
            
            guard let str = String(cString: result, encoding: .utf8) else {
                throw TauqError.utf8Error
            }
            return str
        }
    }
}
