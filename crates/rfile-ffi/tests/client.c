/* SPDX-License-Identifier: GPL-3.0-only */
/* The same header-checked client is compiled against each library in isolation. */
#include <stddef.h>
#include <sys/types.h>
#include "magic.h"
#include <assert.h>
#include <errno.h>
#include <fcntl.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
_Static_assert(MAGIC_VERSION == 548, "API version");
_Static_assert(MAGIC_MIME_TYPE == 0x10 && MAGIC_MIME_ENCODING == 0x400, "MIME flags");
_Static_assert(MAGIC_PARAM_MAGWARN_MAX == 9, "parameter IDs");
static const char *database;
static void *independent(void *ignored) {
    (void)ignored;
    magic_t h=magic_open(MAGIC_MIME_TYPE);assert(h);
    assert(magic_load(h,database)==0);
    for(int i=0;i<32;i++){const char *s=magic_buffer(h,"hello\n",6);assert(s&&strcmp(s,"text/plain")==0);}
    magic_close(h);return NULL;
}
int main(int argc,char **argv) {
    assert(argc==3);database=argv[1];
    assert(magic_version()==548);magic_close(NULL);
    assert(strcmp(magic_error(NULL),"Magic database is not open")==0);
    assert(magic_errno(NULL)==EINVAL&&magic_getflags(NULL)==-1);
    magic_t h=magic_open(0);assert(h);
    assert(!magic_buffer(h,"x",1));assert(strcmp(magic_error(h),"no magic files loaded")==0);
    assert(magic_load(h,database)==0);assert(magic_error(h)==NULL);
    size_t p=0;assert(magic_getparam(h,MAGIC_PARAM_INDIR_MAX,&p)==0&&p==50);
    p=65536;assert(magic_setparam(h,MAGIC_PARAM_INDIR_MAX,&p)==0);assert(magic_getparam(h,0,&p)==0&&p==0);
    p=50;assert(magic_setparam(h,0,&p)==0);
    errno=0;assert(magic_getparam(h,-1,&p)==-1&&errno==EINVAL);
    assert(magic_getmaxparam(MAGIC_PARAM_ELF_SHSIZE_MAX)==65535);
    assert(magic_setflags(h,MAGIC_MIME_TYPE)==0);
    assert(strcmp(magic_buffer(h,"hello\n",6),"text/plain")==0);
    assert(magic_load_buffers(h,NULL,NULL,0)==-1);assert(strcmp(magic_buffer(h,"hello\n",6),"text/plain")==0);
    FILE *f=fopen(database,"rb");assert(f);assert(fseek(f,0,SEEK_END)==0);long end=ftell(f);assert(end>0);rewind(f);
    size_t length=(size_t)end;void *bytes=malloc(length);assert(bytes);assert(fread(bytes,1,length,f)==length);fclose(f);
    void *copy=malloc(length);assert(copy);memcpy(copy,bytes,length);
    void *buffers[1]={bytes};size_t sizes[1]={length};assert(magic_load_buffers(h,buffers,sizes,1)==0);
    assert(memcmp(bytes,copy,length)==0);free(copy);
#ifdef RFILE_OWNED_DATABASES
    free(bytes);bytes=NULL; /* Rust-specific stronger lifetime guarantee. */
#endif
    assert(strcmp(magic_buffer(h,"hello\n",6),"text/plain")==0);
    int fd=open(argv[2],O_RDONLY);assert(fd>=0);assert(lseek(fd,1,SEEK_SET)==1);
    assert(magic_descriptor(h,fd));assert(lseek(fd,0,SEEK_CUR)==1);assert(fcntl(fd,F_GETFD)>=0);close(fd);
    int pipes[2];assert(pipe(pipes)==0);assert(write(pipes[1],"hello\n",6)==6);close(pipes[1]);
    assert(strcmp(magic_descriptor(h,pipes[0]),"text/plain")==0);assert(fcntl(pipes[0],F_GETFD)>=0);close(pipes[0]);
    assert(magic_buffer(h,NULL,0));
    assert(magic_getpath(database,0)==database);
    magic_close(h);free(bytes);
    pthread_t threads[4];for(size_t i=0;i<4;i++)assert(pthread_create(&threads[i],NULL,independent,NULL)==0);
    for(size_t i=0;i<4;i++)assert(pthread_join(threads[i],NULL)==0);
    puts("C ownership/state/concurrency client passed");return 0;
}
