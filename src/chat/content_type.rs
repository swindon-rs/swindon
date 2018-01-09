use std::str::from_utf8;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    Absent,
    Valid,
    Invalid,
}

pub fn check_json<'x, I>(headers: I) -> ContentType
    where I: Iterator<Item=(&'x str, &'x [u8])>,
{
    use self::ContentType::*;
    let mut cur = Absent;
    for (name, value) in headers {
        if name.eq_ignore_ascii_case("Content-Type") {
            match cur {
                Absent => {
                    let mut chunks = value.split(|&x| x == b';');
                    let first = from_utf8(chunks.next().unwrap());
                    if first.is_err() {
                        info!("Invalid content type {:?}, must be json",
                            String::from_utf8_lossy(value));
                        return Invalid;
                    }
                    let mime = first.unwrap().trim();
                    if !mime.eq_ignore_ascii_case("application/json") {
                        info!("Invalid content type {:?}, must be json",
                            String::from_utf8_lossy(value));
                        return Invalid;
                    }
                    for chunk in chunks {
                        let mut pair = chunk.split(|&x| x == b'=');
                        match from_utf8(pair.next().unwrap()) {
                            Ok(s) if s.trim() == "charset" => {
                                let charset = pair.next()
                                    .and_then(|c| from_utf8(c).ok())
                                    .map(|c| c.trim());
                                match charset {
                                    Some("utf-8") | Some("utf8") => {}
                                    _ => {
                                        info!("Invalid content type {:?}, \
                                            charset must be utf-8",
                                            String::from_utf8_lossy(value));
                                        return Invalid;
                                    }
                                }
                            }
                            Ok(..) => {}
                            Err(..) => {}
                        }
                    }
                    cur = Valid;
                }
                Valid | Invalid => {
                    info!("Invalid content type, duplicate header");
                    return Invalid;
                }
            }
        }
    }
    return cur;
}

#[cfg(test)]
mod test {
    use super::check_json;
    use super::ContentType::*;

    #[test]
    fn simple() {
        assert_eq!(check_json(vec![
            ("Content-Type", &b"application/json"[..])
        ].into_iter()), Valid);
    }
    #[test]
    fn charset() {
        assert_eq!(check_json(vec![
            ("Content-Type", &b"application/json; charset=utf-8"[..]),
        ].into_iter()), Valid);
    }

    #[test]
    fn bad_charset() {
        assert_eq!(check_json(vec![
            ("Content-Type", &b"application/json; charset=windows-1251"[..]),
        ].into_iter()), Invalid);
    }

    #[test]
    fn duplicate() {
        assert_eq!(check_json(vec![
            ("Content-Type", &b"application/json; charset=utf-8"[..]),
            ("Content-Type", &b"application/json; charset=utf-8"[..]),
        ].into_iter()), Invalid);
    }

    #[test]
    fn extra_kwargs() {
        assert_eq!(check_json(vec![
            ("Content-Type", &b"application/json; xx=yy; charset=utf-8"[..]),
        ].into_iter()), Valid);
    }

    #[test]
    fn extra_non_utf8() {
        assert_eq!(check_json(vec![
            ("Content-Type", &b"application/json; xx=\x99; charset=utf-8"[..]),
        ].into_iter()), Valid);
    }

    #[test]
    fn non_utf8_charset() {
        assert_eq!(check_json(vec![
            ("Content-Type", &b"application/json; charset=\x99"[..]),
        ].into_iter()), Invalid);
    }
}
