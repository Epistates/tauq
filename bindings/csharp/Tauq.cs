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

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
        private static extern IntPtr tauq_to_tbf(string input, out UIntPtr out_len);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        private static extern IntPtr tauq_tbf_to_json(IntPtr data, UIntPtr len);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        private static extern IntPtr tauq_tbf_to_tauq(IntPtr data, UIntPtr len);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        private static extern void tauq_free_buffer(IntPtr ptr, UIntPtr len);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        private static extern UIntPtr tauq_get_last_error(IntPtr buffer, UIntPtr size);

        private static string GetLastErrorMessage()
        {
            UIntPtr len = tauq_get_last_error(IntPtr.Zero, UIntPtr.Zero);
            if (len == UIntPtr.Zero) return "Unknown error";

            IntPtr buffer = Marshal.AllocHGlobal((int)len.ToUInt32() + 1);
            try
            {
                tauq_get_last_error(buffer, new UIntPtr(len.ToUInt32() + 1));
                return Marshal.PtrToStringAnsi(buffer); // Errors are ASCII/UTF-8
            }
            finally
            {
                Marshal.FreeHGlobal(buffer);
            }
        }

        /// <summary>
        /// Parses Tauq source string to JSON string.
        /// </summary>
        public static string ToJson(string tauqSource)
        {
            if (string.IsNullOrEmpty(tauqSource)) return "";

            IntPtr ptr = tauq_to_json(tauqSource);
            if (ptr == IntPtr.Zero)
            {
                throw new InvalidOperationException(GetLastErrorMessage());
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
        /// Executes Tauq Query (TQQ) source string to JSON string.
        /// </summary>
        public static string ExecQuery(string tqqSource, bool safeMode = false)
        {
            if (string.IsNullOrEmpty(tqqSource)) return "";

            IntPtr ptr = tauq_exec_query(tqqSource, safeMode);
            if (ptr == IntPtr.Zero)
            {
                throw new InvalidOperationException(GetLastErrorMessage());
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
        public static string Minify(string tauqSource)
        {
            if (string.IsNullOrEmpty(tauqSource)) return "";

            IntPtr ptr = tauq_minify(tauqSource);
            if (ptr == IntPtr.Zero)
            {
                throw new InvalidOperationException(GetLastErrorMessage());
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
        public static string ToTauq(string jsonSource)
        {
            if (string.IsNullOrEmpty(jsonSource)) return "";

            IntPtr ptr = json_to_tauq_c(jsonSource);
            if (ptr == IntPtr.Zero)
            {
                throw new InvalidOperationException(GetLastErrorMessage());
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
        /// Converts Tauq or JSON string to TBF bytes.
        /// </summary>
        public static byte[] ToTbf(string input)
        {
            if (string.IsNullOrEmpty(input)) return Array.Empty<byte>();

            IntPtr ptr = tauq_to_tbf(input, out UIntPtr len);
            if (ptr == IntPtr.Zero)
            {
                throw new InvalidOperationException(GetLastErrorMessage());
            }

            try
            {
                int length = (int)len.ToUInt32();
                byte[] bytes = new byte[length];
                Marshal.Copy(ptr, bytes, 0, length);
                return bytes;
            }
            finally
            {
                tauq_free_buffer(ptr, len);
            }
        }

        /// <summary>
        /// Converts TBF bytes to JSON string.
        /// </summary>
        public static string TbfToJson(byte[] data)
        {
            if (data == null || data.Length == 0) return "";

            IntPtr unmanagedPointer = Marshal.AllocHGlobal(data.Length);
            try
            {
                Marshal.Copy(data, 0, unmanagedPointer, data.Length);
                IntPtr ptr = tauq_tbf_to_json(unmanagedPointer, new UIntPtr((uint)data.Length));
                if (ptr == IntPtr.Zero)
                {
                    throw new InvalidOperationException(GetLastErrorMessage());
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
            finally
            {
                Marshal.FreeHGlobal(unmanagedPointer);
            }
        }

        /// <summary>
        /// Converts TBF bytes to Tauq string.
        /// </summary>
        public static string TbfToTauq(byte[] data)
        {
            if (data == null || data.Length == 0) return "";

            IntPtr unmanagedPointer = Marshal.AllocHGlobal(data.Length);
            try
            {
                Marshal.Copy(data, 0, unmanagedPointer, data.Length);
                IntPtr ptr = tauq_tbf_to_tauq(unmanagedPointer, new UIntPtr((uint)data.Length));
                if (ptr == IntPtr.Zero)
                {
                    throw new InvalidOperationException(GetLastErrorMessage());
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
            finally
            {
                Marshal.FreeHGlobal(unmanagedPointer);
            }
        }

        private static string PtrToStringUtf8(IntPtr ptr)
        {
            if (ptr == IntPtr.Zero) return null;
            int len = 0;
            while (Marshal.ReadByte(ptr, len) != 0) len++;
            if (len == 0) return "";
            byte[] bytes = new byte[len];
            Marshal.Copy(ptr, bytes, 0, len);
            return Encoding.UTF8.GetString(bytes);
        }
    }
}
