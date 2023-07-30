# `titlelist`

Reads a link list file (one link per line) and fetches the page title for each
link. This utility is helpful to generate a human-readable list of links,
displaying their titles instead of bare URLs.

Usage:

```none
Usage: titlelist [OPTIONS]

Options:
  -f, --file <FILE>
          Path of the file that contains the URLs, one per line. Unless this
          option is set, readsfrom the standard input

  -t, --template <TEMPLATE>
          Template. Use `%title` and `%url` as placeholders.

          Default is `%title <%url>`.

      --skip-when-no-title
          Doesn't emit links if the page doesn't have a title. By default, this
          is set to `false` and if a page doesn't have a title,
          `@@@ NO TITLE @@@` will be used

  -h, --help
          Print help (see a summary with '-h')
```

Example usage:

```none
$ cat links.txt
https://google.com

$ cat links.txt | titlelist > out.txt
$ cat out.txt
Google <https://google.com>
```
