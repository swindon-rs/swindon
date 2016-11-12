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
    writeln!(buf, "")?;
    writeln!(buf, "routing:")?;
    for (idx, route) in routes.iter().enumerate() {
        let hname = format!("h{}", idx);
        let mut pair = route.splitn(2, '=');
        match (pair.next().unwrap(), pair.next()) {
            (_path, None) => {
                writeln!(buf, "  localhost: {}", hname)?;
                writeln!(buf, "  devd.io: {}", hname)?;
            }
            _ => unimplemented!(),
        }
    }
    writeln!(buf, "")?;
    writeln!(buf, "handlers:")?;
    for (idx, route) in routes.iter().enumerate() {
        let hname = format!("h{}", idx);
        let mut pair = route.splitn(2, '=');
        match (pair.next().unwrap(), pair.next()) {
            (path, None) => {
                writeln!(buf, "")?;
                writeln!(buf, "  h{}: !Static", idx)?;
                writeln!(buf, "    mode: relative_to_route")?;
                writeln!(buf, "    path: {:?}", path)?;
                writeln!(buf, "    text-charset: utf-8")?;
            }
            _ => unimplemented!(),
        }
    }
    Ok(())
}
