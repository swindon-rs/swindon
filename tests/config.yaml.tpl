_VARS:
  - &LISTEN ${listen_address}
  - &DEBUG_ROUTING ${debug_routing}

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

  localhost/empty.gif: empty_gif
  localhost/empty-w-headers.gif: empty_gif_w_headers
  localhost/empty-w-content-length.gif: empty_gif_w_clen

  localhost/static-file: single_file
  localhost/missing-file: missing_file
  localhost/no-permission: no-permission

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
    path: /work/tests/assets/permission.txt
    content-type: text/no/permission

# session-pools:

# http-destinations: {}
