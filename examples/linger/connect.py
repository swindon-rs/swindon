import random
import socket
import argparse
import time


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("-n", "--connections", type=int, default=1000)
    ap.add_argument("--ips", type=int, default=100)
    opt = ap.parse_args()
    sockets = []
    for i in range(opt.connections):
        ip = "127.0.0.{}".format(random.randint(1, 100))
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.connect((ip, 8080))
        sockets.append(s)
        if (i+1) % 1000 == 0:
            print(i+1)
    print("Done")
    while True:
        time.sleep(1000)


if __name__ == '__main__':
    main()
