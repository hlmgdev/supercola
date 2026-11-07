# Gera uma imagem pequena para os testes, sem depender de nada instalado na VM.
import struct
import sys
import zlib


def bloco(tipo, dados):
    return struct.pack(">I", len(dados)) + tipo + dados + struct.pack(">I", zlib.crc32(tipo + dados))


LADO = 32
linhas = b"".join(b"\0" + b"\xff\x40\x00" * LADO for _ in range(LADO))
cabecalho = struct.pack(">IIBBBBB", LADO, LADO, 8, 2, 0, 0, 0)
with open(sys.argv[1], "wb") as saida:
    saida.write(b"\x89PNG\r\n\x1a\n")
    saida.write(bloco(b"IHDR", cabecalho))
    saida.write(bloco(b"IDAT", zlib.compress(linhas)))
    saida.write(bloco(b"IEND", b""))
