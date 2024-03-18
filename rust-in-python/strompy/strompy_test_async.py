import asyncio
import aiofiles
import strompy


async def pipe_bytes(writer):
    async with aiofiles.open('op.json', mode='rb', buffering=1000) as file:
        while True:
            bytes = await file.read(16)
            if len(bytes) == 0:
                break
            await strompy.feed_bytes(writer, bytes)
        print('Done reading!')


async def poll_strompy(reader):
    while True:
        res = await strompy.poll_next(reader)
        if res is None:
            break
        print(f'Result: {res}')


async def main():
    writer, reader = strompy.channel()
    test1 = asyncio.create_task(pipe_bytes(writer))
    test2 = asyncio.create_task(poll_strompy(reader))

    await asyncio.gather(test1, test2)
    await test1

asyncio.run(main())

