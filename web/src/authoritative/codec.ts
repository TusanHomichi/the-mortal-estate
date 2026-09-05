/** Bytes cross the Rust schema boundary before JavaScript sees any fields. */
type CodecExports = WebAssembly.Exports & {
  memory: WebAssembly.Memory;
  codec_reserve(length: number): number;
  codec_decode(nameLength: number): number;
  codec_output(): number;
  codec_output_length(): number;
  codec_protocol_minor(): number;
  codec_control_version(): number;
  codec_control_limit(): number;
};

export class WireCodec {
  private constructor(private readonly exports: CodecExports) {}

  static async create(bytes: BufferSource): Promise<WireCodec> {
    const { instance } = await WebAssembly.instantiate(bytes, {});
    return new WireCodec(instance.exports as CodecExports);
  }

  get protocolMinor(): number { return this.exports.codec_protocol_minor(); }

  get controlVersion(): number { return this.exports.codec_control_version(); }
  get controlLimit(): number { return this.exports.codec_control_limit(); }

  decode<T>(name: string, raw: string | Uint8Array): T {
    const encoder = new TextEncoder();
    const label = encoder.encode(name);
    const input = typeof raw === "string" ? encoder.encode(raw) : raw;
    const offset = this.exports.codec_reserve(label.length + input.length);
    if (offset === 0) throw new Error("wire input exceeds the codec limit");
    const memory = new Uint8Array(this.exports.memory.buffer);
    memory.set(label, offset);
    memory.set(input, offset + label.length);
    if (this.exports.codec_decode(label.length) !== 1) throw new Error("wire document refused");
    const output = new Uint8Array(this.exports.memory.buffer,
      this.exports.codec_output(), this.exports.codec_output_length());
    return JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(output)) as T;
  }
}
