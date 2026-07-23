using System;
using System.Runtime.InteropServices;
using System.Text;

namespace Tavra
{
    /// <summary>
    /// A parsed/decoded Tavra document. Opaque — this binding covers the
    /// same core round-trip surface as tavra-c (parse/format, binary
    /// encode/decode, envelope seal/open); no field-level inspection.
    /// Dispose to free the native handle promptly, or let the finalizer
    /// do it.
    /// </summary>
    public sealed class TavraDocument : IDisposable
    {
        private readonly TavValueHandle _handle;

        private TavraDocument(TavValueHandle handle)
        {
            _handle = handle;
        }

        public void Dispose() => _handle.Dispose();

        /// <summary>Parses UTF-8 <c>.tav</c> text into a document.</summary>
        public static TavraDocument Parse(string text)
        {
            byte[] data = Encoding.UTF8.GetBytes(text);
            int rc = NativeMethods.tav_parse(data, (UIntPtr)data.Length, out TavValueHandle handle);
            if (rc != 0)
            {
                handle.Dispose();
                throw TavraException.FromLastError();
            }
            return new TavraDocument(handle);
        }

        /// <summary>Decodes a canonical <c>.tavb</c> document.</summary>
        public static TavraDocument Decode(byte[] data)
        {
            int rc = NativeMethods.tav_decode(data, (UIntPtr)data.Length, out TavValueHandle handle);
            if (rc != 0)
            {
                handle.Dispose();
                throw TavraException.FromLastError();
            }
            return new TavraDocument(handle);
        }

        /// <summary>Formats this document as canonical <c>.tav</c> text.</summary>
        public string Format()
        {
            int rc = NativeMethods.tav_format(_handle, out IntPtr outData, out UIntPtr outLen);
            if (rc != 0) throw TavraException.FromLastError();
            return Encoding.UTF8.GetString(ReadAndFreeBytes(outData, outLen));
        }

        /// <summary>Encodes this document as canonical <c>.tavb</c>.</summary>
        public byte[] Encode()
        {
            int rc = NativeMethods.tav_encode(_handle, out IntPtr outData, out UIntPtr outLen);
            if (rc != 0) throw TavraException.FromLastError();
            return ReadAndFreeBytes(outData, outLen);
        }

        /// <summary>Seals this document unencrypted, optionally compressed and/or signed.</summary>
        public byte[] SealNone(bool compress = false, byte[]? signWith = null)
        {
            CheckKeyLength(signWith, nameof(signWith));
            int rc = NativeMethods.tav_seal_none(_handle, compress, signWith, out IntPtr outData, out UIntPtr outLen);
            if (rc != 0) throw TavraException.FromLastError();
            return ReadAndFreeBytes(outData, outLen);
        }

        /// <summary>Seals this document with a raw 32-byte XChaCha20-Poly1305 key.</summary>
        public byte[] SealKey(byte[] key, bool compress = false, byte[]? signWith = null)
        {
            CheckKeyLength(key, nameof(key));
            CheckKeyLength(signWith, nameof(signWith));
            int rc = NativeMethods.tav_seal_key(_handle, key, compress, signWith, out IntPtr outData, out UIntPtr outLen);
            if (rc != 0) throw TavraException.FromLastError();
            return ReadAndFreeBytes(outData, outLen);
        }

        /// <summary>Seals this document with an Argon2id-derived key from a password.</summary>
        public byte[] SealPassword(byte[] password, bool compress = false, byte[]? signWith = null)
        {
            CheckKeyLength(signWith, nameof(signWith));
            int rc = NativeMethods.tav_seal_password(_handle, password, (UIntPtr)password.Length, compress, signWith, out IntPtr outData, out UIntPtr outLen);
            if (rc != 0) throw TavraException.FromLastError();
            return ReadAndFreeBytes(outData, outLen);
        }

        /// <summary>Opens an unencrypted <c>.tave</c> document.</summary>
        public static TavraDocument OpenNone(byte[] data, byte[]? verifyKey = null)
        {
            CheckKeyLength(verifyKey, nameof(verifyKey));
            int rc = NativeMethods.tav_open_none(data, (UIntPtr)data.Length, verifyKey, out TavValueHandle handle);
            if (rc != 0)
            {
                handle.Dispose();
                throw TavraException.FromLastError();
            }
            return new TavraDocument(handle);
        }

        /// <summary>Opens a <c>.tave</c> document encrypted with a raw 32-byte key.</summary>
        public static TavraDocument OpenKey(byte[] data, byte[] key, byte[]? verifyKey = null)
        {
            CheckKeyLength(key, nameof(key));
            CheckKeyLength(verifyKey, nameof(verifyKey));
            int rc = NativeMethods.tav_open_key(data, (UIntPtr)data.Length, key, verifyKey, out TavValueHandle handle);
            if (rc != 0)
            {
                handle.Dispose();
                throw TavraException.FromLastError();
            }
            return new TavraDocument(handle);
        }

        /// <summary>Opens a <c>.tave</c> document encrypted with an Argon2id-derived password key.</summary>
        public static TavraDocument OpenPassword(byte[] data, byte[] password, byte[]? verifyKey = null)
        {
            CheckKeyLength(verifyKey, nameof(verifyKey));
            int rc = NativeMethods.tav_open_password(data, (UIntPtr)data.Length, password, (UIntPtr)password.Length, verifyKey, out TavValueHandle handle);
            if (rc != 0)
            {
                handle.Dispose();
                throw TavraException.FromLastError();
            }
            return new TavraDocument(handle);
        }

        /// <summary>Generates a fresh 32-byte XChaCha20-Poly1305 key.</summary>
        public static byte[] GenerateKey()
        {
            byte[] key = new byte[32];
            NativeMethods.tav_genkey(key);
            return key;
        }

        /// <summary>Generates a fresh Ed25519 signing key seed and its matching public key.</summary>
        public static (byte[] Secret, byte[] Public) GenerateSigningKey()
        {
            byte[] secret = new byte[32];
            byte[] pub = new byte[32];
            NativeMethods.tav_gensignkey(secret, pub);
            return (secret, pub);
        }

        private static void CheckKeyLength(byte[]? key, string paramName)
        {
            if (key != null && key.Length != 32)
                throw new ArgumentException($"{paramName} must be exactly 32 bytes, got {key.Length}", paramName);
        }

        private static byte[] ReadAndFreeBytes(IntPtr ptr, UIntPtr len)
        {
            int length = checked((int)len);
            byte[] result = new byte[length];
            if (length > 0)
            {
                Marshal.Copy(ptr, result, 0, length);
            }
            NativeMethods.tav_free_bytes(ptr, len);
            return result;
        }
    }
}
