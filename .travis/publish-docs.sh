#!/bin/bash

publish_docs() {
    pip install sphinx docutils ghp-import --user &&
    pip install -r docs/requirements.txt --user &&
    make html -C docs SPHINXBUILD=~/.local/bin/sphinx-build &&
    ~/.local/bin/ghp-import -n docs/_build/html &&
    git push -fq https://${GH_TOKEN}@github.com/${TRAVIS_REPO_SLUG}.git gh-pages
} && publish_docs
