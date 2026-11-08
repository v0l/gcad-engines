using System;
using System.IO;
using System.Linq;

static class GcadCacheCheck
{
    static int failures;

    static void Expect(bool ok, string what)
    {
        Console.WriteLine((ok ? "ok   " : "FAIL ") + what);
        if (!ok) failures++;
    }

    static int Main(string[] args)
    {
        var tool = args[0];
        var project = args[1];
        var assets = Path.Combine(project, "Assets", "models");
        Directory.CreateDirectory(Path.Combine(assets, "body"));
        File.WriteAllText(Path.Combine(assets, "hero.gcad"), "include body/shape.gcad\n");
        File.WriteAllText(Path.Combine(assets, "body", "shape.gcad"), "blob 0,0,0 r=2,2,2\nremesh 0.6\n");
        var cache = GcadCache.CacheDir(project);
        var hero = Path.Combine(assets, "hero.gcad");
        var flags = GcadCache.Flags("--height 1");

        var first = GcadCache.Sync(tool, cache, hero, flags);
        Expect(first.Error == null, "first sync: " + first.Error);
        Expect(!first.Cached && first.Written, "first sync builds and writes hero.gcad.glb");
        Expect(File.Exists(Path.Combine(cache, first.Key + ".glb")), "Library/gcad-cache holds " + first.Key + ".glb");
        var stamp = File.GetLastWriteTimeUtc(first.Glb);

        var second = GcadCache.Sync(tool, cache, hero, flags);
        Expect(second.Cached && !second.Written && second.Key == first.Key, "second sync is a cache hit and leaves the GLB alone");
        Expect(File.GetLastWriteTimeUtc(first.Glb) == stamp, "so Unity sees no change and does not reimport");

        File.Delete(first.Glb);
        var restored = GcadCache.Sync(tool, cache, hero, flags);
        Expect(restored.Cached && restored.Written, "a deleted GLB comes back from the cache with no build");

        var taller = GcadCache.Sync(tool, cache, hero, GcadCache.Flags("--height 2"));
        Expect(!taller.Cached && taller.Key != first.Key, "other import flags build again");

        File.WriteAllText(Path.Combine(assets, "body", "shape.gcad"), "blob 0,0,0 r=2,2,5\nremesh 0.6\n");
        var edited = GcadCache.Sync(tool, cache, hero, flags);
        Expect(!edited.Cached && edited.Written && edited.Key != first.Key, "an edited include builds again");

        var sources = GcadCache.SourcesUnder(Path.Combine(project, "Assets")).Select(Path.GetFileName).OrderBy(n => n).ToArray();
        Expect(sources.SequenceEqual(new[] { "hero.gcad", "shape.gcad" }), "sources found: " + string.Join(",", sources));

        File.WriteAllText(Path.Combine(assets, "broken.gcad"), "frobnicate 3\n");
        var broken = GcadCache.Sync(tool, cache, Path.Combine(assets, "broken.gcad"), flags);
        Expect(broken.Error != null && broken.Error.Contains("frobnicate"), "a bad line is reported");

        return failures == 0 ? 0 : 1;
    }
}
