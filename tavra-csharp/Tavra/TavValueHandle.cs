using System;
using System.Runtime.InteropServices;

namespace Tavra
{
    /// <summary>
    /// Owns a native <c>TavValue*</c> handle (an opaque parsed/decoded
    /// document root). Frees it via <c>tav_value_free</c> on dispose or
    /// finalization.
    /// </summary>
    internal sealed class TavValueHandle : SafeHandle
    {
        public TavValueHandle() : base(IntPtr.Zero, true)
        {
        }

        public override bool IsInvalid => handle == IntPtr.Zero;

        protected override bool ReleaseHandle()
        {
            NativeMethods.tav_value_free(handle);
            return true;
        }
    }
}
