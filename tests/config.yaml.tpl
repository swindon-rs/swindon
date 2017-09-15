_VARS:
  - &LISTEN ${listen_address}
  - &DEBUG_ROUTING ${debug_routing}
  - &PROXY_ADDRESS ${proxy_address}
  - &SPOOL_ADDRESS1 ${spool_address1}
  - &SPOOL_ADDRESS2 ${spool_address2}
  - &SPOOL_ADDRESS3 ${spool_address3}
  - &SPOOL_ADDRESS4 ${spool_address4}

listen:
- *LISTEN

# These are defaults
#
# max_connections: 1000
# pipeline_depth: 2
# listen_error_timeout: 100ms

server_name: swindon/func-tests
debug-routing: *DEBUG_ROUTING

# Configure all possible routing?
routing:

  ### !EmptyGif routes ###
  localhost/empty.gif: empty_gif
  localhost/empty-w-headers.gif: empty_gif_w_headers
  localhost/empty-w-content-length.gif: empty_gif_w_clen

  ### !SingleFile routes ###
  localhost/static-file: single_file
  localhost/missing-file: missing_file
  localhost/no-permission: no_permission
  localhost/static-file-headers: extra_headers
  localhost/symlink: single_symlink
  localhost/dev-null: dev_null

  ### !Static routes ###
  localhost/static: static
  localhost/static-w-headers: static_w_headers
  localhost/static-w-ctype: static_w_ctype
  localhost/static-wo-charset: static_wo_charset
  localhost/static-w-hostname: static_w_hostname
  localhost/static-w-index: static_w_index
  localhost/static-wo-index: static_wo_index
  localhost/static-autoindex: static_autoindex
  localhost/static-no-permission: static_no_permission

  ### !VersionedStatic routes ###
  localhost/versioned: versioned
  localhost/versioned-fallback: versioned-fallback

  # TODO: add overlapping routes:
  #   /static: !Proxy & /static/file: !SingleFile

  ### !Proxy routes ###
  localhost/proxy: proxy
  localhost/proxy-w-prefix: proxy_w_prefix
  localhost/proxy-w-ip-header: proxy_w_ip_header
  localhost/proxy-w-request-id: proxy_w_request_id
  localhost/proxy-w-host: proxy_w_host
  localhost/proxy-w-timeout: proxy_w_timeout

  ### !SwindonLattice compatibility routes ###
  localhost/swindon-chat: swindon_chat
  localhost/swindon-chat-w-timeouts: swindon_chat_w_timeouts
  localhost/swindon-chat-w-client-timeout: swindon_chat_w_client_timeout

  ### !SwindonLattice routes ###
  localhost/swindon-lattice: swindon_lattice
  localhost/swindon-lattice-w-timeouts: swindon_lattice_w_timeouts
  localhost/swindon-lattice-w-client-timeout: swindon_lattice_w_client_timeout

  ### !WebsocketEcho routes ###
  localhost/websocket-echo: websocket_echo

  ### !BaseRedirect routes ###
  example.com: base_redirect

  ### !StripWWWRedirect routes ###
  www.example.com: strip_www_redirect

  ### !Authorized routes ###
  localhost/auth/local: empty_gif
  localhost/auth/by-header: empty_gif

authorization:
  localhost/auth/local: only-127-0-0-1
  localhost/auth/by-header: by-header

# Configure all possible handlers?
handlers:
  # Allowed handlers are: SwindonLattice, Static, SingleFile, Proxy,
  #   EmptyGif, HttpBin, WebsocketEcho;

  ### EmptyGif handlers ###
  empty_gif: !EmptyGif
  empty_gif_w_headers: !EmptyGif
    extra-headers:
      X-Some-Header: some value
  empty_gif_w_clen: !EmptyGif
    extra-headers:
      Content-Type: image/other
      Content-Length: 100500

  ### SingleFile handlers ###

  single_file: !SingleFile
    path: ${TESTS_DIR}/assets/static_file.txt
    content-type: text/plain
  missing_file: !SingleFile
    path: ${TESTS_DIR}/assets/missing_file.txt
    content-type: text/is/missing
  no_permission: !SingleFile
    path: /tmp/no-permission.txt
    content-type: text/no/permission
  extra_headers: !SingleFile
    path: ${TESTS_DIR}/assets/static_file.txt
    content-type: text/plain
    extra-headers:
      X-Extra-Header: "extra value"
      X-Bad-Header: "bad header\r\n"
  single_symlink: !SingleFile
    path: ${TESTS_DIR}/assets/link.txt
    content-type: text/plain
  dev_null: !SingleFile
    path: /dev/null
    content-type: text/plain

  ### Static handlers ###

  static: !Static
    path: ${TESTS_DIR}/assets/
  static_w_headers: !Static
    path: ${TESTS_DIR}/assets/
    extra-headers:
      X-Some-Header: some value
  static_w_ctype: !Static
    path: ${TESTS_DIR}/assets/
    extra-headers:
      Content-Type: something/other
  static_wo_charset: !Static
    path: ${TESTS_DIR}/assets/
    text_charset: null
  static_w_hostname: !Static
    mode: with_hostname
    path: ${TESTS_DIR}/assets/
  static_w_index: !Static
    path: ${TESTS_DIR}/assets/index
    index-files:
    - index.html
  static_wo_index: !Static
    path: ${TESTS_DIR}/assets/index
  static_autoindex: !Static
    path: ${TESTS_DIR}/assets
    generate-index: true
  static_no_permission: !Static
    path: /tmp

  versioned: !VersionedStatic
    versioned-root: ${TESTS_DIR}/hashed
    plain-root: ${TESTS_DIR}/assets
    version-arg: "r"
    version-split: [2, 6]
    version-chars: lowercase_hex
    fallback-to-plain: never

  versioned-fallback: !VersionedStatic
    versioned-root: ${TESTS_DIR}/hashed
    plain-root: ${TESTS_DIR}/assets
    version-arg: "r"
    version-split: [2, 6]
    version-chars: lowercase_hex
    fallback-to-plain: always

  ### Proxy handlers ###

  proxy: !Proxy
    destination: proxy_dest/
  proxy_w_prefix: !Proxy
    destination: proxy_dest/prefix
  proxy_w_ip_header: !Proxy
    destination: proxy_dest
    ip-header: X-Some-Header
  proxy_w_request_id: !Proxy
    destination: proxy_dest
  proxy_w_host: !Proxy
    destination: proxy_host
  proxy_w_timeout: !Proxy
    destination: proxy_timeout
  swindon_proxy: !Proxy
    destination: swindon_http_dest

  ### SwindonLattice compatibility handlers ###
  swindon_chat: !SwindonLattice
    compatibility: v0.5.4
    session_pool: swindon_pool_old
    http_route: swindon_proxy
    message_handlers:
      "*": swindon_chat_dest/
      prefixed.*: swindon_chat_dest/with-prefix
      rxid.*: swindon_chat_w_rxid/
  swindon_chat_w_timeouts: !SwindonLattice
    compatibility: v0.5.4
    session_pool: pool_w_timeouts_old
    message_handlers:
      "*": swindon_chat_dest/
  swindon_chat_w_client_timeout: !SwindonLattice
    compatibility: v0.5.4
    session_pool: swindon_pool_old
    http_route: swindon_proxy
    message_handlers:
      "*": swindon_chat_w_timeout/

  ### SwindonLattice handlers ###
  swindon_lattice: !SwindonLattice
    session_pool: swindon_pool_new
    http_route: swindon_proxy
    message_handlers:
      "*": swindon_lattice_dest/
      prefixed.*: swindon_lattice_dest/with-prefix
      rxid.*: swindon_lattice_w_rxid/
  swindon_lattice_w_timeouts: !SwindonLattice
    session_pool: pool_w_timeouts_new
    message_handlers:
      "*": swindon_lattice_dest/
  swindon_lattice_w_client_timeout: !SwindonLattice
    session_pool: swindon_pool_new
    http_route: swindon_proxy
    message_handlers:
      "*": swindon_lattice_w_timeout/

  ### WebsocketEcho handlers ###
  websocket_echo: !WebsocketEcho

  ### BaseRedirect handler ###

  base_redirect: !BaseRedirect
    redirect-to-domain: localhost

  ### StripWWWRedirect handler
  strip_www_redirect: !StripWWWRedirect

session-pools:
  swindon_pool_old:
    listen:
    - *SPOOL_ADDRESS1
    inactivity_handlers: []
    ### defaults: ###
    # pipeline_depth: 2
    # max_connections: 1000
    # listen_error_timeout: 100ms
    # max_payload_size: 10485760
  swindon_pool_new:
    listen:
    - *SPOOL_ADDRESS3
    inactivity_handlers: []
  pool_w_timeouts_old:
    listen:
    - *SPOOL_ADDRESS2
    inactivity_handlers:
    - swindon_chat_dest
    new_connection_idle_timeout: 1s
    client_min_idle_timeout: 1s
    client_max_idle_timeout: 10s
  pool_w_timeouts_new:
    listen:
    - *SPOOL_ADDRESS4
    inactivity_handlers:
    - swindon_lattice_dest
    new_connection_idle_timeout: 1s
    client_min_idle_timeout: 1s
    client_max_idle_timeout: 10s

http-destinations:
  ### Proxy destintations ###
  proxy_dest:
    addresses:
    - *PROXY_ADDRESS
    request-id-header: X-Request-Id

  proxy_host:
    override-host-header: swindon.proxy.example.org
    addresses:
    - *PROXY_ADDRESS
  proxy_timeout:
    addresses:
    - *PROXY_ADDRESS
    max-request-timeout: 1s

  ### SwindonLattice compatibility destinations ###
  swindon_http_dest:
    addresses:
    - *PROXY_ADDRESS
  swindon_chat_dest:
    override-host-header: swindon.internal
    addresses:
    - *PROXY_ADDRESS
  swindon_chat_w_rxid:
    override-host-header: swindon.internal
    request-id-header: X-Request-Id
    addresses:
    - *PROXY_ADDRESS
  swindon_chat_w_timeout:
    override-host-header: swindon.internal
    max-request-timeout: 1s
    addresses:
    - *PROXY_ADDRESS

  ### SwindonLattice destinations ###
  swindon_lattice_dest:
    override-host-header: swindon.internal
    addresses:
    - *PROXY_ADDRESS
  swindon_lattice_w_rxid:
    override-host-header: swindon.internal
    request-id-header: X-Request-Id
    addresses:
    - *PROXY_ADDRESS
  swindon_lattice_w_timeout:
    override-host-header: swindon.internal
    max-request-timeout: 1s
    addresses:
    - *PROXY_ADDRESS

authorizers:

  only-127-0-0-1: !SourceIp
    allowed-network: only-127-0-0-1

  by-header: !SourceIp
    allowed-network: goog
    forwarded-ip-header: X-Real-Ip
    accept-forwarded-headers-from: only-127-0-0-1

networks:
  only-127-0-0-1:
  - 127.0.0.1
  goog:
  - 8.0.0.0/8
