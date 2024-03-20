import asyncio
import aiofiles
import strompy

async def feed(writer):
    async with aiofiles.open('op.json', mode='rb', buffering=1000) as file:
        while True:
            bytes = await file.read(16)
            if len(bytes) == 0:
                break
            await strompy.feed_bytes(writer, bytes)
        print('Done reading!')


async def poll(reader):
    while True:
        res = await strompy.poll_next(reader)
        if res is None:
            break
        print(f'Result: {res}')


async def main():
    writer, reader = strompy.channel()
    write = asyncio.create_task(feed(writer))
    poll = asyncio.create_task(poll(reader))

    await asyncio.gather(write, poll)

asyncio.run(main())

