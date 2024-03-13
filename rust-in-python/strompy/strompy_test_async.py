import asyncio
import aiofiles
import strompy


async def pipe_bytes(writer):
    async with aiofiles.open('op.json', mode='rb', buffering=1000) as file:
        while True:
            bytes = await file.read(1000)
            print('Feeding writer: ' + bytes.decode())
            if len(bytes) == 0:
                break
            await strompy.feed_bytes(writer, bytes)


async def poll_strompy(reader):
    res = await strompy.poll_next(reader)
    print('Result:')
    print(res)


async def main():
    reader, writer = strompy.channel()
    test1 = asyncio.create_task(pipe_bytes(writer))
    test2 = asyncio.create_task(poll_strompy(reader))

    await asyncio.gather(test1, test2)
    await test1

asyncio.run(main())

