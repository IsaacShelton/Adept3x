
/*
   Example of ability to enable/disable preprocessor.

   All major C compilers suffer from this horrible problem of
   having to use cryptic identifiers in headers to reduce
   the likelihood of user-defined macros interfering.

   None of them have a way to declare
   functions, structs, etc. without being vulnerable
   to malicious definitions created by the user or
   other headers.

   This is also extremely bad when you consider that
   parameter names in files like `<stdio.h>` are all
   prefixed with underscores or are in _Reserved_case.

   This litters language server completions with superfluous
   prefix underscores and uppercase letters that detract
   from the actual meaning of completions. (Not good)

   Even when compilers attempt regular mitigations in standard
   headers, they can still easily become malfunctioning
   if someone defines a macro with the wrong name.

   Pretty crazy if you think about it.

   I don't know the best solution to this problem,
   but a glaringly obvious low-tech solution is having
   the ability to enable/disable the preprocessor.

   In complicated function macros however, it may
   be benefitial to have some way to indicate that
   certain identifiers are immune to replacement.

   I can't believe the major compilers don't have a
   solution for this, take a look at GNU `<vector.h>` and tell
   me it looks sane.

   but I digress.

   We won't suffer from this problem in our standard headers.
*/

/*
    NOTE: Enabling/disabling the preprocessor only affects non-directive lines.

    Otherwise, you'd never be able to re-enable it!

    Perhaps it could use a better name to indicate this, although nothing
    short comes to mind
*/

#ifndef _HEADER_H_INCLUDED
#define _HEADER_H_INCLUDED


#define printf not_printf
#define int 78045345

/*
    NOTE: These work, but due to `#define` importing requiring parsing of all C expressions to determine validity,
    these will trigger a TODO that some C expressions are not implemented yet.

    So in the mean time, we will disable them.
*/
// #define format +
// #define char *^&@#^%*@&%#^)@*#%&_@*#%&
// #define const ,

#pragma adept preprocessor disable

/*
    NOTE: These identifiers would normally be replaced by the
    preprocesser, but since we're smart and disabled it, the
    declaration will proceed unharmed to C compilation
*/
int printf(const char *format, ...);

/*
   Otherwise, we'd have do something ugly like

int printf(const char *__format, ...);

    Which is still vulnerable to redefinition
    of `char` for example
*/

#pragma adept preprocessor enable

#endif // _HEADER_H_INCLUDED
