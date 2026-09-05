//! Synchronous, instance-local byte ABI. No JS schema or binding dependency.
//! A caller writes the reserved input, decodes, copies output, then may reserve
//! again. No Rust reference is held across an exported call.
use std::cell::RefCell;

thread_local! {
    static INPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static OUTPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

#[unsafe(no_mangle)]
pub extern "C" fn codec_reserve(length: usize) -> usize {
    OUTPUT.with(|output| output.borrow_mut().clear());
    INPUT.with(|input| {
        let mut input = input.borrow_mut();
        input.clear();
        if length > crate::MAX_SERVER_ENVELOPE_BYTES + 64 {
            return 0;
        }
        input.resize(length, 0);
        input.as_mut_ptr() as usize
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn codec_decode(name_length: usize) -> u32 {
    let result = INPUT.with(|input| {
        let input = input.borrow();
        let (name, payload) = input
            .split_at_checked(name_length)
            .ok_or_else(|| crate::ProtocolError::new("invalid decoder length"))?;
        let name = std::str::from_utf8(name)
            .map_err(|_| crate::ProtocolError::new("invalid decoder name"))?;
        crate::decode_document(name, payload)
    });
    OUTPUT.with(|output| {
        let mut output = output.borrow_mut();
        output.clear();
        match result {
            Ok(bytes) => {
                *output = bytes;
                1
            }
            Err(_) => 0, // Never return rejected payloads or secrets in errors.
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn codec_output() -> usize {
    OUTPUT.with(|output| output.borrow().as_ptr() as usize)
}

#[unsafe(no_mangle)]
pub extern "C" fn codec_output_length() -> usize {
    OUTPUT.with(|output| output.borrow().len())
}

#[unsafe(no_mangle)]
pub extern "C" fn codec_protocol_minor() -> u32 {
    u32::from(crate::PROTOCOL_MINOR)
}
