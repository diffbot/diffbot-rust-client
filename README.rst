Diffbot_ API client for Rust_
=============================

.. image:: https://travis-ci.org/chris-morgan/diffbot-rust-client.png?branch=master
   :target: https://travis-ci.org/chris-morgan/diffbot-rust-client

Installation
------------

This library depends on rust-http_.

There is no especially convenient way to install this library at present;
rustpkg in its present `is on the way out`_ and this library does not actually
install correctly with it (due to a rustpkg bug in how it handles the rust-http
dependency).

Until further notice, then, installation is a fairly manual process.

Installation goes somewhat along these lines::

   git clone https://github.com/chris-morgan/rust-http.git
   git clone https://github.com/diffbot/diffbot-rust-client.git
   cd diffbot-rust-client
   make deps all

(The ``deps`` rule is equivalent to ``cd ../rust-http && make``.)

The library files (``*.so``, ``*.dylib``, ``*.dll``, ``*.rlib``) in both cases
will go into the repository's ``build`` directory; hence if you are depending
upon the Diffbot Rust client library, your own ``rustc`` invocation will need
something like ``-L ../rust-http -L ../diffbot-rust-client`` in it.
Alternatively, copy the library files out of that build directory elsewhere.

In your own Rust source file, load the diffbot crate with the likes of::

   extern mod diffbot = "diffbot#1.0";

You're now ready to use the Diffbot Rust client!

Usage
-----

Please see the project documentation at
http://www.rust-ci.org/chris-morgan/diffbot-rust-client/doc/diffbot/.

.. _Diffbot: http://diffbot.com/
.. _Rust: http://www.rust-lang.org/
.. _rust-http: https://gihub.com/chris-morgan/rust-http
.. _is on the way out:
   https://mail.mozilla.org/pipermail/rust-dev/2014-January/008224.html
