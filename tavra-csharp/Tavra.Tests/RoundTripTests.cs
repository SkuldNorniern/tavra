using System.Text;
using Tavra;
using Xunit;

namespace Tavra.Tests
{
    public class RoundTripTests
    {
        [Fact]
        public void ParseAndFormatRoundTrip()
        {
            using var doc = TavraDocument.Parse("name = \"demo\"\nport = 8080\n");
            string formatted = doc.Format();
            Assert.Contains("name = \"demo\"", formatted);
            Assert.Contains("port = 8080", formatted);
        }

        [Fact]
        public void BinaryRoundTrip()
        {
            using var doc = TavraDocument.Parse("a = 1\nb = \"x\"\n");
            byte[] encoded = doc.Encode();
            using var decoded = TavraDocument.Decode(encoded);
            Assert.Equal(doc.Format(), decoded.Format());
        }

        [Fact]
        public void EnvelopeKeyModeWithSignatureRoundTrip()
        {
            using var doc = TavraDocument.Parse("secret = \"value\"\n");
            byte[] key = TavraDocument.GenerateKey();
            var (secret, publicKey) = TavraDocument.GenerateSigningKey();

            byte[] sealed_ = doc.SealKey(key, compress: true, signWith: secret);
            using var opened = TavraDocument.OpenKey(sealed_, key, publicKey);
            Assert.Equal(doc.Format(), opened.Format());
        }

        [Fact]
        public void WrongKeyFailsWithTavraException()
        {
            using var doc = TavraDocument.Parse("secret = \"value\"\n");
            byte[] key = TavraDocument.GenerateKey();
            byte[] wrongKey = TavraDocument.GenerateKey();

            byte[] sealed_ = doc.SealKey(key);
            var ex = Assert.Throws<TavraException>(() => TavraDocument.OpenKey(sealed_, wrongKey));
            Assert.Contains("decryption failed", ex.Message);
        }

        [Fact]
        public void PasswordModeRoundTrip()
        {
            using var doc = TavraDocument.Parse("secret = \"value\"\n");
            byte[] password = Encoding.UTF8.GetBytes("correct horse battery staple");

            byte[] sealed_ = doc.SealPassword(password);
            using var opened = TavraDocument.OpenPassword(sealed_, password);
            Assert.Equal(doc.Format(), opened.Format());
        }

        [Fact]
        public void WrongPasswordFails()
        {
            using var doc = TavraDocument.Parse("secret = \"value\"\n");
            byte[] sealed_ = doc.SealPassword(Encoding.UTF8.GetBytes("right"));
            Assert.Throws<TavraException>(() => TavraDocument.OpenPassword(sealed_, Encoding.UTF8.GetBytes("wrong")));
        }

        [Fact]
        public void InvalidKeyLengthThrowsArgumentException()
        {
            using var doc = TavraDocument.Parse("a = 1\n");
            byte[] badKey = new byte[16];
            Assert.Throws<ArgumentException>(() => doc.SealKey(badKey));
        }

        [Fact]
        public void MalformedTavTextThrowsTavraException()
        {
            Assert.Throws<TavraException>(() => TavraDocument.Parse("not valid [ tav"));
        }
    }
}
