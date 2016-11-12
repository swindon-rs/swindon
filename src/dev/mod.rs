use std::fmt::{self, Write};

pub fn generate_config(port: u16, routes: &[String]) -> String {
    let mut buffer = String::new();
    _generate_config(&mut buffer, port, routes).unwrap();
    return buffer;
}

fn _generate_config(buf: &mut String, port: u16, routes: &[String])
    -> Result<(), fmt::Error>
{
    writeln!(buf, "listen: [127.0.0.1:{}]", port)?;
    Ok(())
}
