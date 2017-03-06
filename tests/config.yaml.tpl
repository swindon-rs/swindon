_VARS:
  - &LISTEN ${listen_address}
  - &DEBUG_ROUTING ${debug_routing}
  - &PROXY_ADDRESS ${proxy_address}
  - &SPOOL_ADDRESS1 ${spool_address1}
  - &SPOOL_ADDRESS2 ${spool_address2}

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
  localhost/static-w-hostname: static_w_hostname

  # TODO: add overlapping routes:
  #   /static: !Proxy & /static/file: !SingleFile

  ### !Proxy routes ###
  localhost/proxy: proxy
  localhost/proxy-w-prefix: proxy_w_prefix
  localhost/proxy-w-ip-header: proxy_w_ip_header

  ### !SwindonChat routes ###
  localhost/swindon-chat: swindon_chat
  localhost/swindon-chat-w-timeouts: swindon_chat_w_timeouts

  ### !WebsocketEcho routes ###
  localhost/websocket-echo: websocket_echo

  ### !BaseRedirect routes ###
  example.com: base_redirect

  ### !StripWWWRedirect routes ###
  www.example.com: strip_www_redirect

# Configure all possible handlers?
handlers:
  # Allowed handlers are: SwindonChat, Static, SingleFile, Proxy,
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
    path: /work/tests/assets/static_file.txt
    content-type: text/plain
  missing_file: !SingleFile
    path: /work/tests/assets/missing_file.txt
    content-type: text/is/missing
  no_permission: !SingleFile
    path: /no-permission.txt
    content-type: text/no/permission
  extra_headers: !SingleFile
    path: /work/tests/assets/static_file.txt
    content-type: text/plain
    extra-headers:
      X-Extra-Header: "extra value"
      X-Bad-Header: "bad header\r\n"
  single_symlink: !SingleFile
    path: /work/tests/assets/link.txt
    content-type: text/plain
  dev_null: !SingleFile
    path: /dev/null
    content-type: text/plain

  ### Static handlers ###

  static: !Static
    path: /work/tests/assets/
  static_w_headers: !Static
    path: /work/tests/assets/
    extra-headers:
      X-Some-Header: some value
  static_w_ctype: !Static
    path: /work/tests/assets/
    extra-headers:
      Content-Type: something/other
  static_w_hostname: !Static
    mode: with_hostname
    path: /work/tests/assets/

  ### Proxy handlers ###

  proxy: !Proxy
    destination: proxy_dest/
  proxy_w_prefix: !Proxy
    destination: proxy_dest/prefix
  proxy_w_ip_header: !Proxy
    destination: proxy_dest
    ip-header: X-Some-Header
  swindon_proxy: !Proxy
    destination: swindon_http_dest

  ### SwindonChat handlers ###
  swindon_chat: !SwindonChat
    session_pool: swindon_pool
    http_route: swindon_proxy
    message_handlers:
      "*": swindon_chat_dest/
      prefixed.*: swindon_chat_dest/with-prefix
  swindon_chat_w_timeouts: !SwindonChat
    session_pool: pool_w_timeouts
    message_handlers:
      "*": swindon_chat_dest/

  ### WebsocketEcho handlers ###
  websocket_echo: !WebsocketEcho

  ### BaseRedirect handler ###

  base_redirect: !BaseRedirect
    redirect-to-domain: localhost

  ### StripWWWRedirect handler
  strip_www_redirect: !StripWWWRedirect

session-pools:
  swindon_pool:
    listen:
    - *SPOOL_ADDRESS1
    inactivity_handlers:
    - swindon_chat_dest/
    ### defaults: ###
    # pipeline_depth: 2
    # max_connections: 1000
    # listen_error_timeout: 100ms
    # max_payload_size: 10485760
  pool_w_timeouts:
    listen:
    - *SPOOL_ADDRESS2
    inactivity_handlers:
    - swindon_chat_dest
    new_connection_idle_timeout: 1s
    client_min_idle_timeout: 1s
    client_max_idle_timeout: 10s

http-destinations:
  ### Proxy destintations ###
  proxy_dest:
    addresses:
    - *PROXY_ADDRESS

  ### SwindonChat destinations ###
  swindon_http_dest:
    addresses:
    - *PROXY_ADDRESS
  swindon_chat_dest:
    addresses:
    - *PROXY_ADDRESS
