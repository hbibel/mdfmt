# Supplemental dependency notices

These locked registry packages omit their license text. Copies retain the original
copyrights from the exact VCS revisions recorded in their published packages:

- fast-glob 1.1.2: [LICENSE at cd1157ebe949c07c2e08ccf7b96f753dfc08e7bd](https://github.com/oxc-project/fast-glob/blob/cd1157ebe949c07c2e08ccf7b96f753dfc08e7bd/LICENSE).
- nodejs-built-in-modules 1.0.0: [LICENSE at 9117efa38fe088325e8a1036a8ddea7f3546c1ac](https://github.com/oxc-project/nodejs-built-in-modules/blob/9117efa38fe088325e8a1036a8ddea7f3546c1ac/LICENSE).

Release tooling uses these version-specific supplements only when the package has
no packaged notice. Dependency updates require rechecking the source and license;
a missing notice for an unknown version fails explicitly rather than inventing
copyright text. Generated RE2JS/Unicode data has separate notices under `src/`.
