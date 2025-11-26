using System;
using System.Runtime.InteropServices;
using System.Text;

namespace Tauq
{
    /// <summary>
    /// Tauq (τq) Bindings for .NET
    /// </summary>
    public static class TauqInterop
    {
        private const string LibName = "tauq";

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
        private static extern IntPtr tauq_to_json(string input);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
        private static extern IntPtr tauq_exec_query(string input, [MarshalAs(UnmanagedType.I1)] bool safe_mode);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
        private static extern IntPtr tauq_minify(string input);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
        private static extern IntPtr json_to_tauq_c(string input);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        private static extern void tauq_free_string(IntPtr ptr);

        /// <summary>
        /// Parses Tauq source string to JSON string.
        /// </summary>
        /// <param name="tauqSource">The Tauq input.</param>
        /// <returns>JSON string representation.</returns>
        /// <exception cref="InvalidOperationException">If parsing fails.</exception>
        public static string ToJson(string tauqSource)
        {
            if (string.IsNullOrEmpty(tauqSource)) return "";

            IntPtr ptr = tauq_to_json(tauqSource);
            if (ptr == IntPtr.Zero)
            {
                throw new InvalidOperationException("Failed to parse Tauq.");
            }

            try
            {
                // Assuming UTF-8, but C# strings are UTF-16.
                // tauq_to_json returns a C-string (char*), which Marshaling handles as Ansi (default) or needs specific handling.
                // Rust strings are UTF-8. Marshal.PtrToStringAnsi does system default page (often not UTF-8 on Windows).
                // Correct way for cross-platform UTF-8:
                return PtrToStringUtf8(ptr);
            }
            finally
            {
                tauq_free_string(ptr);
            }
        }

        /// <summary>
        /// Executes Tauq Query (TQQ) source string to JSON string.
        /// </summary>
        /// <param name="tqqSource">The Tauq Query input.</param>
        /// <param name="safeMode">If true, disables unsafe directives like !run.</param>
        /// <returns>JSON string representation.</returns>
        public static string ExecQuery(string tqqSource, bool safeMode = false)
        {
            if (string.IsNullOrEmpty(tqqSource)) return "";

            IntPtr ptr = tauq_exec_query(tqqSource, safeMode);
            if (ptr == IntPtr.Zero)
            {
                throw new InvalidOperationException("Failed to execute Tauq Query.");
            }

            try
            {
                return PtrToStringUtf8(ptr);
            }
            finally
            {
                tauq_free_string(ptr);
            }
        }

        /// <summary>
        /// Minify Tauq source to single-line Tauq string.
        /// </summary>
        /// <param name="tauqSource">The Tauq input.</param>
        /// <returns>Minified Tauq string.</returns>
        public static string Minify(string tauqSource)
        {
            if (string.IsNullOrEmpty(tauqSource)) return "";

            IntPtr ptr = tauq_minify(tauqSource);
            if (ptr == IntPtr.Zero)
            {
                throw new InvalidOperationException("Failed to minify Tauq.");
            }

            try
            {
                return PtrToStringUtf8(ptr);
            }
            finally
            {
                tauq_free_string(ptr);
            }
        }

        /// <summary>
        /// Formats JSON string to Tauq.
        /// </summary>
        /// <param name="jsonSource">The JSON input.</param>
        /// <returns>Tauq string representation.</returns>
        public static string ToTauq(string jsonSource)
        {
            if (string.IsNullOrEmpty(jsonSource)) return "";

            IntPtr ptr = json_to_tauq_c(jsonSource);
            if (ptr == IntPtr.Zero)
            {
                throw new InvalidOperationException("Failed to format JSON.");
            }

            try
            {
                return PtrToStringUtf8(ptr);
            }
            finally
            {
                tauq_free_string(ptr);
            }
        }

        private static string PtrToStringUtf8(IntPtr ptr)
        {
            if (ptr == IntPtr.Zero) return null;
            
            // Find length
            int len = 0;
            while (Marshal.ReadByte(ptr, len) != 0) len++;
            
            if (len == 0) return "";
            
            byte[] bytes = new byte[len];
            Marshal.Copy(ptr, bytes, 0, len);
            return Encoding.UTF8.GetString(bytes);
        }
    }
}
