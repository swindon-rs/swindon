import pytest


def test_empty_config(check_config):
    assert check_config() != ""


def test_no_listen(check_config):
    err = check_config("""
        routing: {}
        handlers: {}
    """)
    assert err != ''


# XXX: all this values (except 1) are valid yaml booleans
@pytest.mark.parametrize("debug_routing", [
    "yes", "on", "y", "1",
    "no", "off", "n", "0",
    ])
def test_debug_variants(check_config, debug_routing):
    err = check_config("""
        debug_routing: {}
    """.format(debug_routing))
    assert (
        ".debug_routing: Can't parse value: provided string"
        " was not `true` or `false`"
        )in err


def test_invalid_listen(check_config):
    err = check_config("""
        listen: 127.0.0.1:80
        routing: {}
        handlers: {}
    """)
    assert ".listen[0]: Expected sequence, got string" in err

    err = check_config("""
        listen:
        - 127.0.0.1:80
        - null
        routing: {}
        handlers: {}
    """)
    assert ".listen[1]: Expected scalar, got Null" in err

    err = check_config("""
        listen:
        - - 127.0.0.1
          - 80
        routing: {}
        handlers: {}
    """)
    assert ".listen[0]: Expected scalar, got Seq" in err
    # XXX: naming: Expected "sequence" but got "Seq"


def test_no_proxy_destination(check_config):
    cfg = """
        listen:
        - 127.0.0.1:8080
        routing:
            localhost:/abc: abc
        handlers:
            abc: !Proxy {}
    """
    err = check_config(cfg)
    assert ".handlers.abc.destination: Expected scalar, got Null" in err


def test_no_handler(check_config):
    cfg = """
        listen:
        - 127.0.0.1:8080
        routing:
            localhost/path: unkown-handler
        handlers:
            known-handler: !EmptyGif
    """
    err = check_config(cfg)
    assert err != ''


def test_invalid_routing(check_config):
    err = check_config("""
        listen:
        - 127.0.0.1:80
        routing:
        - host:port/path: handler
    """)
    assert '.routing: Mapping expected' in err
    # XXX: total inconsistency: missing 'got ...' phrase.


def test_invalid_handlers(check_config):
    err = check_config("""
        listen:
        - 1.2.3.4:5
        handlers:
        - handler: !EmptyGif
    """)
    assert '.handlers: Mapping expected' in err
