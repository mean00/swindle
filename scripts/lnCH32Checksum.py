#/usr/bin/python3
import sys
import xxhash;


header_size=4
total_header_size=header_size+2*4
#
#_____________
def usage():
    print("lnCH32Checksum : Add a checksum to the fw to check its integrity");
    print("\tlnCH32Checksum.py intput_bin_file checksumed_bin_file\n");
#
#_____________
def write32(f,v):
    n_byte = v.to_bytes(4,byteorder='little')
    f.write(n_byte)
#
#_____________
if(len(sys.argv)!=3):
    usage();
    exit(-1);

infile=sys.argv[1];
outfile=sys.argv[2];

print("lnCH32Checksum:  checksuming %s => %s " %(infile,outfile))


print("Reading file..")
file=open(infile,"rb")
content=file.read(-1)
file.close();

binSize=len(content)

print("%d bytes read " % (binSize))

binSize-=total_header_size # Skip 4 first words = MSP / Reset / size / xxhash

payload = content[total_header_size:]

lnHash=xxhash.xxh32(payload,0x100).intdigest()

print("digest %x " % (lnHash))


result=open(outfile,"wb")
result.write(content[0:4])
write32(result,binSize)
write32(result,lnHash)
result.write(payload)
result.close()
print("Done")


