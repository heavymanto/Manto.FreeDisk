// Manto.FreeDisk.Cli — gemello C# di mfd-cli per il confronto Rust-vs-C# (vedi docs/ARCHITECTURE.md)
// Walker parallelo minimale idiomatico .NET. Punto architetturale: EnumerateFileSystemInfos
// ottiene le dimensioni dai dati di enumerazione (FindFirstFile/FindNextFile) SENZA una
// stat per file — su Windows è il percorso veloce in userspace prima del motore $MFT.
// Logica identica a node/mfd-scan/walker-node.mjs: somma dimensioni logiche, conta file/cartelle,
// salta i reparse point (junction/symlink), conta gli errori di accesso.
// Uso: dotnet Manto.FreeDisk.Cli.dll <path>   (stopwatch interno, avvio runtime escluso)
using System.Collections.Concurrent;
using System.Diagnostics;

if (args.Length < 1)
{
    Console.Error.WriteLine("uso: Manto.FreeDisk.Cli <path> [loops]");
    return 2;
}
string root = Path.GetFullPath(args[0]);
// loops opzionale (default 1): esperimento "processo lungo" — ripete il walk nello STESSO
// processo per separare i costi una-tantum (JIT del codice dell'app, avvio ThreadPool)
// dal throughput a regime. Con loops=1 l'output è identico a prima (una riga RESULT).
int loops = args.Length > 1 && int.TryParse(args[1], out var n) && n > 0 ? n : 1;

(long ms, long bytes, long files, long dirs, long errors) WalkOnce(string root)
{
    long bytes = 0, files = 0, dirs = 0, errors = 0;
    var sw = Stopwatch.StartNew();

    // BFS con contatore "pending": pattern worker-pool, stesso identico algoritmo del walker Node
    var queue = new BlockingCollection<string>(new ConcurrentQueue<string>());
    long pending = 1;
    queue.Add(root);

    int workersN = Environment.ProcessorCount;
    var workers = new Task[workersN];
    for (int i = 0; i < workersN; i++)
    {
        workers[i] = Task.Run(() =>
        {
            foreach (string dir in queue.GetConsumingEnumerable())
            {
                try
                {
                    foreach (FileSystemInfo e in new DirectoryInfo(dir).EnumerateFileSystemInfos())
                    {
                        if ((e.Attributes & FileAttributes.ReparsePoint) != 0) continue; // junction/symlink
                        if (e is DirectoryInfo d)
                        {
                            Interlocked.Increment(ref dirs);
                            Interlocked.Increment(ref pending);
                            queue.Add(d.FullName);
                        }
                        else
                        {
                            Interlocked.Increment(ref files);
                            Interlocked.Add(ref bytes, ((FileInfo)e).Length);
                        }
                    }
                }
                catch (UnauthorizedAccessException) { Interlocked.Increment(ref errors); }
                catch (IOException) { Interlocked.Increment(ref errors); }   // include DirectoryNotFoundException, FileNotFound
                if (Interlocked.Decrement(ref pending) == 0) queue.CompleteAdding();
            }
        });
    }
    Task.WaitAll(workers);
    sw.Stop();
    return ((long)sw.Elapsed.TotalMilliseconds, bytes, files, dirs, errors);
}

for (int k = 1; k <= loops; k++)
{
    var (ms, bytes, files, dirs, errors) = WalkOnce(root);
    if (loops == 1)
        Console.WriteLine($"RESULT ms={ms} bytes={bytes} files={files} dirs={dirs} errors={errors}");
    else
        Console.WriteLine($"ITER {k}/{loops} ms={ms} bytes={bytes} files={files} dirs={dirs} errors={errors}");
}
return 0;
