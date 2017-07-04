Websocket Shutdown Codes
========================

Here are codes that you can see in ``onclose`` event handler:

.. code-block:: javascript

   var ws = new WebSocket(..)
   os.onclose = function(ev) {
        console.log(ev.code, ev.reason)
   }


Our custom codes (and reasons):

* ``4001``, ``session_pool_stopped`` -- session pool is closed,
  basically this means that this specific
  application is not supported by this server any more. This message may be
  received at any time.
* ``4400``, ``backend_error`` -- no websockets allowed at this route
* ``4401``, ``backend_error`` -- unauthorized (i.e. no cookie or other
  authentication data)
* ``4403``, ``backend_error`` -- forbidden (i.e. authorized but not allowed
* ``4404``, ``backend_error`` -- route not found (http status: Not Found)
* ``4410``, ``backend_error`` -- route not found (http status: Gone)
* ``4500``, ``backend_error`` -- internal server error when authorizing
* ``4503``, ``backend_error`` -- temporary error, unable to authorize

Bacically we have reserved these status codes to correspond to HTTP error
codes returned from backend. But we only guarantee to propagate codes
described above, because other ones may impose security vulnerability.

* ``4400-4499``, ``backend_error`` -- for HTTP codes 400-499
* ``4500-4599``, ``backend_error`` -- for HTTP codes 500-599

These errors only propagate on connection authorization. When single request
fails we respond with ``["error"...]`` as websocket message.
