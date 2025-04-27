#include <termios.h>
#include <unistd.h>
#include <stdlib.h>
#include <stdio.h>

void tty_atexit(void);
int tty_reset(void);
void tty_raw(void);
void fatal(char *mess);

static struct termios orig_termios;
static int ttyfd = 0;

void raw_mod()
{
    if (!isatty(ttyfd))
        fatal("not on a tty");

    if (tcgetattr(ttyfd, &orig_termios) < 0)
        fatal("can't get tty settings");

    if (atexit(tty_atexit) != 0)
        fatal("atexit: can't register tty reset");

    tty_raw();
}

void tty_atexit(void)
{
    tty_reset();
}

int tty_reset(void)
{
    if (tcsetattr(ttyfd, TCSAFLUSH, &orig_termios) < 0)
        return -1;
    return 0;
}

void tty_raw(void)
{
    struct termios raw;

    raw = orig_termios;
    raw.c_iflag &= ~(BRKINT | ICRNL | INPCK | ISTRIP | IXON);
    raw.c_oflag &= ~(OPOST);
    raw.c_cflag |= (CS8);
    raw.c_lflag &= ~(ECHO | ICANON | IEXTEN);
    raw.c_cc[VMIN] = 5;
    raw.c_cc[VTIME] = 8;
    raw.c_cc[VMIN] = 0;
    raw.c_cc[VTIME] = 0;
    raw.c_cc[VMIN] = 2;
    raw.c_cc[VTIME] = 0;
    raw.c_cc[VMIN] = 0;
    raw.c_cc[VTIME] = 8;

    if (tcsetattr(ttyfd, TCSAFLUSH, &raw) < 0)
        fatal("can't set raw mode");
}

void fatal(char *message)
{
    fprintf(stderr, "fatal error: %s\n", message);
    exit(1);
}