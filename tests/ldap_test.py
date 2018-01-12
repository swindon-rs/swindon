import aiohttp


def assert_403(resp, data, debug_routing):
    assert resp.status == 403

async def test_ldap_forbidden(swindon, http_request, debug_routing):
    resp, data = await http_request(swindon.url / 'auth/ldap')
    assert_403(resp, data, debug_routing)
    if debug_routing:
        assert resp.headers['X-Swindon-Authorizer'] == 'ldap'
        assert resp.headers['X-Swindon-Deny'] == 'dlap'
