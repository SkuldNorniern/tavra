using System;
using System.Runtime.InteropServices;

namespace Tavra
{
    /// <summary>
    /// P/Invoke declarations mirroring tavra-c's <c>include/tavra.h</c>
    /// exactly. Every fallible function returns 0 on success, nonzero on
    /// error (check <see cref="tav_last_error"/>).
    /// </summary>
    internal static class NativeMethods
    {
        private const string LibName = "tavra_c";

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern IntPtr tav_last_error();

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern void tav_free_bytes(IntPtr ptr, UIntPtr len);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern void tav_value_free(IntPtr value);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int tav_parse(byte[] data, UIntPtr len, out TavValueHandle outValue);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int tav_format(TavValueHandle value, out IntPtr outData, out UIntPtr outLen);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int tav_encode(TavValueHandle value, out IntPtr outData, out UIntPtr outLen);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int tav_decode(byte[] data, UIntPtr len, out TavValueHandle outValue);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int tav_seal_none(TavValueHandle value, [MarshalAs(UnmanagedType.I1)] bool compress, byte[]? signKey, out IntPtr outData, out UIntPtr outLen);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int tav_seal_key(TavValueHandle value, byte[] key, [MarshalAs(UnmanagedType.I1)] bool compress, byte[]? signKey, out IntPtr outData, out UIntPtr outLen);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int tav_seal_password(TavValueHandle value, byte[] password, UIntPtr passwordLen, [MarshalAs(UnmanagedType.I1)] bool compress, byte[]? signKey, out IntPtr outData, out UIntPtr outLen);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int tav_open_none(byte[] data, UIntPtr len, byte[]? verifyKey, out TavValueHandle outValue);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int tav_open_key(byte[] data, UIntPtr len, byte[] key, byte[]? verifyKey, out TavValueHandle outValue);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int tav_open_password(byte[] data, UIntPtr len, byte[] password, UIntPtr passwordLen, byte[]? verifyKey, out TavValueHandle outValue);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int tav_genkey(byte[] output);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        internal static extern int tav_gensignkey(byte[] outSecret, byte[] outPublic);
    }
}
