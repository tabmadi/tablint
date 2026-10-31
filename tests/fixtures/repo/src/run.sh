#!/bin/sh
# shellcheck disable=SC2086
echo "${#x}" # The count, basically.
cat <<END
# not a comment, it is data; with (parens)
END
