

def test_empty_config(check_config):
    assert check_config() != ""


def test_no_listen(check_config):
    err = check_config("""
        routing: {}
        handlers: {}
    """)
    assert err != ''


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
