import asyncio
import aiofiles
import strompy


async def pipe_bytes(writer):
    async with aiofiles.open('op.json', mode='rb', buffering=100) as file:
        while file.readable():
            await strompy.feed_bytes(writer, file.read(100))
            await asyncio.sleep(1)


async def poll_strompy(reader):
    while True:
        print('Polling Strompy')
        res = await strompy.poll_next(reader)
        await asyncio.sleep(1)
        print(res)


async def main():
    reader, writer = strompy.channel()
    test1 = asyncio.create_task(pipe_bytes(writer))
    test2 = asyncio.create_task(poll_strompy(reader))

    await asyncio.gather(test1, test2)

asyncio.run(main())

