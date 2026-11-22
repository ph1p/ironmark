/* Compiles the public header and calls every entry point. Run by CI:
 *   cargo build && cc -std=c99 -Wall -Wextra -Werror -o smoke tests/c/smoke.c \
 *     target/debug/libironmark.a -lpthread -ldl -lm && ./smoke
 */
#include "../../include/ironmark.h"

#include <stdio.h>
#include <string.h>

static int fail(const char *what, const char *got) {
    fprintf(stderr, "FAIL %s: %s\n", what, got ? got : "(null)");
    return 1;
}

int main(void) {
    char *html = ironmark_render_html("# Hi\n");
    if (!html || strcmp(html, "<h1>Hi</h1>\n") != 0) return fail("render_html", html);
    ironmark_free(html);

    ironmark_options opts = ironmark_options_default();
    opts.flags |= IRONMARK_OPT_DISABLE_RAW_HTML;
    opts.flags &= ~IRONMARK_OPT_HARD_BREAKS;

    /* Not NUL-terminated at len: only the first 9 bytes are read. */
    const char buf[] = "<b>x</b>\nignored";
    html = ironmark_render_html_with_options(buf, 9, &opts);
    if (!html || strcmp(html, "<p>&lt;b&gt;x&lt;/b&gt;</p>\n") != 0)
        return fail("with_options", html);
    ironmark_free(html);

    opts.flags = IRONMARK_OPT_ALL + 1; /* unknown bit */
    html = ironmark_render_html_with_options(buf, 9, &opts);
    if (html) return fail("unknown flag accepted", html);

    html = ironmark_render_html_with_options("*a*", 3, NULL);
    if (!html || strcmp(html, "<p><em>a</em></p>\n") != 0) return fail("null options", html);
    ironmark_free(html);

    ironmark_free(NULL);
    puts("ok");
    return 0;
}
