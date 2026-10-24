/* Test-only reference benchmark, never linked to the Rust product. BSD-2-Clause. */
#define _POSIX_C_SOURCE 200809L
#include <magic.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <time.h>
#include <sys/resource.h>
#include <string.h>
static double now(void){struct timespec t;clock_gettime(CLOCK_MONOTONIC,&t);return t.tv_sec+t.tv_nsec/1e9;}
int main(int argc,char **argv){
 if(argc!=3)return 2;FILE *list=fopen(argv[1],"r");if(!list)return 2;
 unsigned char *samples[512];size_t sizes[512],n=0,total=0;char *line=NULL;size_t cap=0;ssize_t len;
 while((len=getline(&line,&cap,list))!=-1){if(n==512)return 2;if(len&&line[len-1]=='\n')line[--len]=0;FILE *f=fopen(line,"rb");if(!f)return 2;unsigned char *buf=malloc(7*1024*1024);if(!buf)return 2;sizes[n]=fread(buf,1,7*1024*1024,f);samples[n++]=buf;total+=sizes[n-1];fclose(f);}
 free(line);fclose(list);double start=now();magic_t m=magic_open(0);if(!m||magic_load(m,argv[2]))return 2;double cold=now()-start;
 for(size_t i=0;i<n;i++)if(!magic_buffer(m,samples[i],sizes[i]))return 2;
 uint64_t checksum=0;double durations[10];
 for(size_t pass=0;pass<10;pass++){start=now();for(size_t rep=0;rep<3;rep++)for(size_t i=0;i<n;i++){const char *s=magic_buffer(m,samples[i],sizes[i]);if(!s)return 2;for(;*s;s++)checksum+=(unsigned char)*s;}durations[pass]=now()-start;}
 struct rusage r;getrusage(RUSAGE_SELF,&r);printf("{\"engine\":\"c\",\"cold_load_seconds\":%.9f,\"warm_pass_seconds\":[",cold);for(int i=0;i<10;i++)printf("%s%.9f",i?",":"",durations[i]);printf("],\"calls_per_pass\":%zu,\"input_bytes\":%zu,\"checksum\":%llu,\"peak_rss_kib\":%ld}\n",n*3,total,(unsigned long long)checksum,r.ru_maxrss);
 magic_close(m);for(size_t i=0;i<n;i++)free(samples[i]);return 0;
}
