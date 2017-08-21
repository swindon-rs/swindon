import pytest


def test_empty_config(check_config):
    # NOTE: empty config is ok
    assert check_config(returncode=0) == ""


def test_no_listen(check_config):
    err = check_config("""
        routing: {}
        handlers: {}
    """, returncode=0)
    assert err == ''


# XXX: all this values (except 1) are valid yaml booleans
@pytest.mark.parametrize("debug_routing", [
    "yes", "on", "y", "1",
    "no", "off", "n", "0",
    ])
def test_debug_variants(check_config, debug_routing):
    err = check_config("""
        debug_routing: {}
    """.format(debug_routing))
    assert ".debug_routing: bad boolean " in err


def test_invalid_listen(check_config):
    err = check_config("""
        listen: 127.0.0.1:80
        routing: {}
        handlers: {}
    """)
    assert ".listen: sequence expected, got Scalar" in err

    err = check_config("""
        listen:
        - 127.0.0.1:80
        - null
        routing: {}
        handlers: {}
    """)
    assert ".listen[1]: expected string, got Null" in err

    err = check_config("""
        listen:
        - - 127.0.0.1
          - 80
        routing: {}
        handlers: {}
    """)
    assert ".listen[0]: expected string, got Seq" in err
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
    assert ".handlers.abc: missing field `destination`" in err


def test_unknown_proxy_destination(check_config):
    cfg = """
        listen:
        - 127.0.0.1:8080
        routing:
            localhost:/abc: abc
        handlers:
            abc: !Proxy
                destination: unknown-dest
    """
    err = check_config(cfg)
    assert (
        "handler\"abc\": unknown http destination upstream\"unknown-dest\""
        ) in err


def test_unknown_chat_http_route(check_config):
    cfg = """
        routing:
            localhost:/abc: chat
        handlers:
            chat: !SwindonChat
                session-pool: chat
                http-route: unknown-dest
                message-handlers:
                    "*": dummy/
        session-pools:
            chat:
                inactivity-handlers:
                - dummy/
        http-destinations:
            dummy:
                addresses:
                - 1.2.3.4:5
    """
    err = check_config(cfg)
    assert (
        "handler\"chat\": unknown http route handler\"unknown-dest\""
        ) in err


def test_unknown_chat_message_handlers(check_config):
    cfg = """
        routing:
            localhost:/abc: chat
        handlers:
            chat: !SwindonChat
                session-pool: chat
                message-handlers:
                    "*": dummy/
                    "test": unknown-dest/
        session-pools:
            chat:
                inactivity-handlers:
                - dummy/
        http-destinations:
            dummy:
                override-host-header: swindon.internal
                addresses:
                - 1.2.3.4:5
    """
    err = check_config(cfg)
    assert (
        "handler\"chat\": unknown http destination upstream\"unknown-dest\""
        ) in err

    cfg = """
        routing:
            localhost:/abc: chat
        handlers:
            chat: !SwindonChat
                session-pool: chat
                message-handlers:
                    "*": unknown-dest/
                    "tangle.*": dummy/
        session-pools:
            chat:
                inactivity-handlers:
                - dummy/
        http-destinations:
            dummy:
                addresses:
                - 1.2.3.4:5
    """
    err = check_config(cfg)
    assert (
        "handler\"chat\": unknown http destination upstream\"unknown-dest\""
        ) in err

def test_override_host_header_is_set(check_config):
    cfg = """
        routing:
            localhost:/abc: chat
        handlers:
            chat: !SwindonChat
                session-pool: chat
                message-handlers:
                    "*": dummy/
                    "test": unknown-dest/
        session-pools:
            chat:
                inactivity-handlers:
                - dummy/
        http-destinations:
            dummy:
                addresses:
                - 1.2.3.4:5
    """
    err = check_config(cfg)
    assert (
        'validation error: http destination upstream"dummy" '
        'is used in message-handler of handler"chat", '
        'so must contain override-host-header setting.'
        ) in err


def test_unknown_session_pool_dest(check_config):
    cfg = """
        session-pools:
            chat:
                inactivity-handlers:
                - dummy/
    """
    err = check_config(cfg)
    assert (
        "sessionpool\"chat\": unknown http destination upstream\"dummy\""
        ) in err


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
    assert '.routing: mapping expected' in err
    # XXX: total inconsistency: missing 'got ...' phrase.


def test_invalid_handlers(check_config):
    err = check_config("""
        listen:
        - 1.2.3.4:5
        handlers:
        - handler: !EmptyGif
    """)
    assert '.handlers: mapping expected' in err


def test_extra_headers(check_config):
    err = check_config("""
        listen:
        - 1.2.3.4:5
        routing:
            host/path: handler
        handlers:
            handler: !SingleFile
                path: /work/
                content-type: text/plain
                extra-headers:
                    Content-Type: text/html
    """)
    assert (
        '.handlers.handler: Content-Type must be specified as `content-type`'
        ' parameter rather than in `extra-headers`') in err


def test_www_redirect_route_prefix(check_config):
    err = check_config("""
        routing:
            example.com: www_redirect
        handlers:
            www_redirect: !StripWWWRedirect
    """)
    assert (
        "Expected `www.` prefix for StripWWWRedirect handler route:"
        " Host(false, \"example.com\")"
        ) in err


def test_route_path_suffix(check_config):
    err = check_config("""
        routing:
            localhost/some/path/: handler
        handlers:
            handler: !EmptyGif
    """)
    assert (
        "Path must not end with /: Host(false, \"localhost\")"
        " Some(\"/some/path/\") handler\"handler\""
        ) in err


def test_no_inactivity(check_config):
    cfg = """
        routing:
            localhost:/abc: chat
        handlers:
            chat: !SwindonChat
                session-pool: chat
                message-handlers:
                    "*": dummy/
        session-pools:
            chat:
        http-destinations:
            dummy:
                override-host-header: myhost
                addresses:
                - 1.2.3.4:5
    """
    err = check_config(cfg, returncode=0)
    assert err == ''

def test_mixins(check_config):
    err = check_config("""
        routing:
            localhost/some/path: app1-handler
            localhost/other/path: app2-handler
        mixins:
            app1-: app1.yaml
            app2-: app2.yaml
    """, files=dict(
        app1='''
            handlers:
                app1-handler: !EmptyGif
        ''', app2='''
            handlers:
                app2-handler: !EmptyGif
        '''),
        returncode=0)
    assert err == ''

def test_mixins_fingerprint(check_fingerprint):
    fp = check_fingerprint("""
        routing:
            localhost/some/path: app1-handler
            localhost/other/path: app2-handler
        mixins:
            app1-: app1.yaml
            app2-: app2.yaml
    """, files=dict(
        app1='''
            handlers:
                app1-handler: !EmptyGif
        ''', app2='''
            handlers:
                app2-handler: !EmptyGif
        '''))
    assert fp[:39] == b'Config fingerprint: bbcd0ce03fb0d058ecd', repr(fp)

def test_bad_mixins(check_config):
    err = check_config("""
        routing:
            localhost/some/path: handler
            localhost/other/path: app2-handler
        mixins:
            app1-: app1.yaml
            app2-: app2.yaml
    """, files=dict(
        app1='''
            handlers:
                handler: !EmptyGif
        ''', app2='''
            handlers:
                app2-handler: !EmptyGif
        '''))
    assert ('app1.yaml" has handler named "handler" without prefix "app1-"'
        ) in err
