.. highlight:: yaml

.. _handlers:

Handlers
========

.. contents:: Handlers
   :local:


Proxy handler
-------------

.. index:: pair: !Proxy; Handlers


Proxy handler looks like::

  example-chat-http: !Proxy
    ip-header: X-Remote-Ip
    destination: somedest/some/path

Settings:

.. opt:: destination

   (required) The name of the destination and *subpath* where to forward
   request to.

.. opt:: ip-header

   (default is null) Name of the HTTP header where to put source IP address to.

.. opt:: request-id-header

   **Deprecated**, use ``request-id-header`` option in
   :ref:`http-destination <http_destinations>`.

   (default is null) Creates a request id

.. opt:: max-payload-size

   (default ``10MiB``) Maximum payload size that might be send to this
   destination

.. opt:: stream-requests

   (default ``false``) If ``true`` requests bodies might be forwarded to the
   backend before whole body has been read.

   You shouldn't turn this option to ``true`` unless your backend is
   asynchronous too or can start processing request before receiving full body.

.. opt:: response-buffer-size

   (default ``10MiB``) A high water mark of buffering responses in swindon. If
   this number of bytes is reached swindon will stop reading response from a
   backend until client receives some data.

   Here are few tips for tweaking this value:

   1. The size of most of your pages (or other content served through this
      proxy) should be strictly less than this value, to have good performance.
      This holds true even for async backends written in scripting languages.
   2. For non-async backends even if just one page of your site doesn't fit
      the buffer it might make DoS attack super-easy unless this page is
      protected by some rate limit.
   3. Making this limit lower makes sense when you can generate data
      continuously, like fetching data from the database by chunks, or
      decompress data on the fly.
   4. Consider making use cases (1-2) and (3) separate routes with different
      limits.


Static & Single file handlers
-----------------------------

.. index::
   pair: !SingleFile; Handlers
   pair: !Static; Handlers
   pair: !VersionedStatic; Handlers

Handler for serving static files::

   robots-txt: !SingleFile
      path: /www/my-host/robots.txt
      content-type: text/plain

   static-files: !Static
      path: /www/my-host/static

Common Settings
```````````````

.. opt:: pool

   (default: ``default``) Disk pool name to be used to serve this file.

.. opt:: extra-headers

   (optional) Extra HTTP headers to be added to response.

``!SingleFile`` settings:

.. opt:: path

   (required) Path to file to serve.

.. opt:: content-type

   (required) Set Content type for served file.

!Static Settings
````````````````

.. opt:: path

   (required) Path to directory to serve.

.. opt:: mode

   (default: ``relative_to_route``) Sets path resolve mode:

   * ``relative_to_domain_root``
      Use whole URL path as filesystem path to file;

   * ``relative_to_route``
      Use only route suffix/tail as filesystem path to file;

   * ``with_hostname``
      Add hostname as the first directory component

   These pathes, ofcourse, relative to ``path`` setting.

.. opt:: text-charset

   (optional) Sets ``charset`` parameter of ``Content-Type`` header.

.. opt:: strip-host-suffix

   (optional) If ``mode`` is ``with_hostname`` strip the specified suffix
   from the host name before using it as a first component of a directory.
   For example, if ``strip-host-suffix`` is ``example.org`` then URL
   ``http://something.example.com/xx/yy`` will be searched in the directory
   ``something/xx/yy``.

.. opt:: index-files

   (default ``[]``) List of files to be used as a directory index.
   If none of them found (or ``index-files`` is an empty list) the 403 error
   is returned.

   MIME type for index file is guessed just like for any other file.

   Example::

        index-files: ["index.html", "index.htm"]

!VersionedStatic Settings
`````````````````````````

.. opt:: versioned-root

   (required) Root of the directory where versioned files should be served
   from. Basic pattern how files are served from there is::

       <versioned-root>/xx/yyyyyy/filename.ext

   In particular:

   * ``xx/yyyyy`` is a value extracted from ``version-arg``
   * Name of the file is original one, but without path. Name is kept barely
     to debug issues easier.
   * Extension (suffix) of the filename is kept as is to be able to find
     out mime type.

   So for example url ``/img/myimage.jpg?r=deadbeef`` is served from
   ``/versioned-root/de/adbeef/myimage.jpg``.

.. opt:: plain-root

   (optional) When no file found in ``versioned-root`` we may search in
   ``plain-root`` by original filename/path depending on
   ``fallback-to-plain`` setting. This works in a way similar to
   ``Static``.

   It's expected that ``plain-root`` contains files of the latest version of
   an application. And it's main purpose is to serve well-known files like
   ``robots.txt`` or ``crossdomain.xml``.

.. opt:: version-arg

   (required) The query argument to get version from. It's usually some
   short thing like ``r``, ``v``, ``ver``, ``revision``, ``hash``.

.. opt:: version-split

   (required) Parts to split version argument into, to search for a path.
   Sum of all number here must be equal to the length of the version argument,
   we do not support variable length yet.

   For example ``version-split: [2, 6]`` means that value must
   consist of eight characters and that ``myimage.gif?r=deadbeef`` is searched
   in ``de/adbeef`` folder.

.. opt:: version-chars

   (required) Validates version chars allowed in hash string. Currently only
   ``lowercase-hex`` mode is supported.

.. opt:: fallback-to-plain

   (default ``never``) When to fallback to serving files from ``plain-root``.
   We have a very conservative default, it's useful for staging servers where
   you want specifically, For production deployment, you may wish to change it
   to more lenient ones.

.. opt:: fallback-mode

   (default ``relative_to_route``) A mode to serve url if there is no versioned
   file. This directly corresponds to :opt:`mode` of ``!Static``.

.. opt:: text-charset

   (optional) Sets ``charset`` parameter of ``Content-Type`` header.



Swindon Lattice Handler
-----------------------

.. index::
   pair: !SwindonLattice; Handlers

.. index::
   pair: !SwindonChat; Handlers

Swindon lattice handler::

   example-chat: !SwindonLattice
      session-pool: example-chat-session
      http-route: backend/fallback
      message-handlers:
        "*": backend/path

Old name of the handler type is ``SwindonChat`` which is deprecated.

The ``backend/path`` here, i.e. the message handler, should have
:opt:`override-host-header` setting set, so that swindon knows what ``Host``
header to send for RPC requests.

Settings:

.. opt:: session-pool

   (required) Sets session pool to be used with this chat

.. opt:: http-route

   (optional) Sets fallback http route to be used in case when
   URL is accessed with plain http request, not websocket upgrade request.

.. opt:: message-handlers

   (required) Mapping of chat method name patterns to http handlers.

   Allowed patterns of 3 types:

   ``"*"`` -- (required) special "default" pattern; any method with doesn't match
      any other pattern will be sent to this http handler.

   ``"prefix.*"`` -- "glob" pattern matches method name by prefix including dot,
      for instance, pattern ``"chat.*"`` will match::

         chat.send_message
         chat.hello

      but will not match::

         chat_send_message
         chat

      also "chat.send*" is invalid pattern, it will be read as 'exact' pattern,
      however will not work ever because "*" is not allowed in method names.

   ``"exact.pattern"`` -- "exact" pattern, matches whole method name.

   Patterns match order is: "exact" then "glob" otherwise "default".

.. opt:: allow-empty-subprotocol

   (default ``false``) This is backwards compatibility option. If set to true
   it allows connecting without `Sec-WebSocket-Protocol` header.

   **Deprecated** Do not set to ``true`` for new applications.

   .. note::

      By default set to ``true`` in ``SwindonChat``
      (``false`` in ``SwindonLattice``)

   .. versionadded:: v0.5.5


Redirect handlers
-----------------

.. index::
   pair: !BaseRedirect; Handlers
   pair: !StripWWWRedirect; Handlers

``!BaseRedirect`` handler is used for permanent base host redirects::

   routing:
      example.com: new-handler
      example.org: redirect
   handlers:
      redirect: !BaseRedirect
         redirect-to-domain: example.com

      new-handler: !Proxy
         destination: somedest/

.. opt:: redirect-to-domain

   Destination domain to redirect to.

``!StripWWWRedirect`` handler is used redirect to URL without ``www.`` prefix::

   routing:
      example.com: new-handler
      www.example.com: strip-www
   handlers:
      strip-www: !StripWWWRedirect
      example.com: !Proxy
         destination: somedest/

.. note:: Both redirects use *301 Moved Permanently* status code.


WebsocketEcho
-------------

.. index:: pair: !WebsocketEcho; Handlers

Handler for a dummy websocket echo service::

   echo: !WebsocketEcho


Empty GIF handler
-----------------

.. index:: pair: !EmptyGif; Handlers

Empty GIF handler is used to serve static empty pixel gif image::

   empty-gif: !EmptyGif

Seetings:

.. opt:: extra-headers

   Mapping of extra http headers to return in response.

Http bin handler
----------------

.. index:: pair: !HttpBin; Handlers

Serves kind'a request-response testing service, see http://httpbin.org.
