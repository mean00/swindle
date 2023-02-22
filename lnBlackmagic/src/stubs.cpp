#include "lnArduino.h"
extern "C"
{
char *last_strk=NULL;

char *x_strtok_r (  char *s,
	  const char *delim,
	char **lasts,
	int skip_leading_delim)
{
	  char *spanp;
	  int c, sc;
	char *tok;


	if (s == NULL && (s = *lasts) == NULL)
		return (NULL);

	/*
	 * Skip (span) leading delimiters (s += strspn(s, delim), sort of).
	 */
cont:
	c = *s++;
	for (spanp = (char *)delim; (sc = *spanp++) != 0;) {
		if (c == sc) {
			if (skip_leading_delim) {
				goto cont;
			}
			else {
				*lasts = s;
				s[-1] = 0;
				return (s - 1);
			}
		}
	}

	if (c == 0) {		/* no non-delimiter characters */
		*lasts = NULL;
		return (NULL);
	}
	tok = s - 1;

	/*
	 * Scan token (scan for delimiters: s += strcspn(s, delim), sort of).
	 * Note that delim must have one NUL; we stop if we see that, too.
	 */
	for (;;) {
		c = *s++;
		spanp = (char *)delim;
		do {
			if ((sc = *spanp++) == c) {
				if (c == 0)
					s = NULL;
				else
					s[-1] = 0;
				*lasts = s;
				return (tok);
			}
		} while (sc != 0);
	}
	/* NOTREACHED */
}

//-------------------------------


char *
m_strtok (  char *__restrict s,
	  const char *__restrict delim)
{
	return x_strtok_r (s, delim, &last_strk, 1);
}

}
