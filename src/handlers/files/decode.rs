pub fn decode_component(buf: &mut Vec<u8>, component: &str) -> Result<(), ()>
{
    let mut chariter = component.as_bytes().iter();
    while let Some(c) = chariter.next() {
        match *c {
            b'%' => {
                let h = from_hex(*chariter.next().ok_or(())?)?;
                let l = from_hex(*chariter.next().ok_or(())?)?;
                let b = (h << 4) | l;
                if b == 0 || b == b'/' {
                    return Err(());
                }
                buf.push(b);
            }
            0 => return Err(()),
            c => buf.push(c),
        }
    }
    Ok(())
}

fn from_hex(b: u8) -> Result<u8, ()> {
    match b {
        b'0'...b'9' => Ok(b & 0x0f),
        b'a'...b'f' | b'A'...b'F' => Ok((b & 0x0f) + 9),
        _ => Err(())
    }
}
