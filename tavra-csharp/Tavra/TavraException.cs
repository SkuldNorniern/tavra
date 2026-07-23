using System;
using System.Runtime.InteropServices;

namespace Tavra
{
    /// <summary>
    /// Thrown when a tavra-c call returns nonzero. Message comes from
    /// <c>tav_last_error()</c>.
    /// </summary>
    public sealed class TavraException : Exception
    {
        public TavraException(string message) : base(message)
        {
        }

        internal static TavraException FromLastError()
        {
            IntPtr ptr = NativeMethods.tav_last_error();
            string message = ptr == IntPtr.Zero
                ? "unknown Tavra error (no message set)"
                : Marshal.PtrToStringUTF8(ptr) ?? "unknown Tavra error";
            return new TavraException(message);
        }
    }
}
