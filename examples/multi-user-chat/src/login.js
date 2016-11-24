import cookie from 'cookie';


export function get_login() {
    let {swindon_muc_login} = cookie.parse(document.cookie);
    return swindon_muc_login
}
