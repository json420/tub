from hashlib import blake2b
from os import getrandom


def hash_data(data):
    return blake2b(data, digest_size=30).digest()


def random_size():
    return int.from_bytes(getrandom(2), 'little')


def random_data():
    return getrandom(random_size())


def random_entry():
    d = random_data()
    h = hash_data(d)
    s = len(d).to_bytes(8, 'little')
    print(h.hex(), len(d))
    return b''.join([h, s, d])


fp = open('test.btdb', 'xb')

total = 0
for i in range(100_000):
    buf = random_entry()
    total += fp.write(buf)

print(total)
