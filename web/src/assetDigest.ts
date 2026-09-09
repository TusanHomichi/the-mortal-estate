// Content identity verification shared by pixel art and reference packets.
function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}

export async function verifySha256(
  bytes: ArrayBuffer,
  expected: string,
  subtle: SubtleCrypto = globalThis.crypto.subtle,
): Promise<void> {
  const digest = await subtle.digest("SHA-256", bytes);
  if (bytesToHex(new Uint8Array(digest)) !== expected) {
    throw new Error("asset digest does not match the manifest");
  }
}
