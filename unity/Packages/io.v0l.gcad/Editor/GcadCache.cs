using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;

public sealed class GcadCacheResult
{
    public string Source;
    public string Glb;
    public string Key;
    public bool Cached;
    public bool Written;
    public string Error;
}

public static class GcadCache
{
    public static readonly string[] Extensions = { ".gcad", ".gasm" };

    public static string CacheDir(string projectRoot)
    {
        return Path.Combine(projectRoot, "Library", "gcad-cache");
    }

    public static string OutputFor(string source)
    {
        return source + ".glb";
    }

    public static bool IsSource(string path)
    {
        return Extensions.Any(e => path.EndsWith(e, StringComparison.OrdinalIgnoreCase));
    }

    public static string[] Flags(string userData)
    {
        if (string.IsNullOrWhiteSpace(userData)) return new string[0];
        return userData.Split(new[] { ' ', '\t', '\n' }, StringSplitOptions.RemoveEmptyEntries);
    }

    static string Quote(string arg)
    {
        return arg.IndexOfAny(new[] { ' ', '"' }) < 0 ? arg : "\"" + arg.Replace("\"", "\\\"") + "\"";
    }

    public static GcadCacheResult Sync(string tool, string cacheDir, string source, string[] flags)
    {
        var result = new GcadCacheResult { Source = source };
        var args = new List<string>(flags) { "--cache", cacheDir, source };
        var start = new ProcessStartInfo(tool, string.Join(" ", args.Select(Quote)))
        {
            UseShellExecute = false,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            CreateNoWindow = true,
        };
        string stdout, stderr;
        int code;
        try
        {
            using (var process = Process.Start(start))
            {
                var errors = process.StandardError.ReadToEndAsync();
                stdout = process.StandardOutput.ReadToEnd();
                process.WaitForExit();
                stderr = errors.Result;
                code = process.ExitCode;
            }
        }
        catch (Exception e)
        {
            result.Error = tool + ": " + e.Message;
            return result;
        }
        var cached = stdout.Trim();
        if (code != 0 || cached.Length == 0 || !File.Exists(cached))
        {
            result.Error = tool + " exited " + code + ": " + stderr.Trim();
            return result;
        }
        result.Cached = stderr.StartsWith("cached ", StringComparison.Ordinal);
        result.Key = Path.GetFileNameWithoutExtension(cached);
        result.Glb = OutputFor(source);
        var bytes = File.ReadAllBytes(cached);
        if (!File.Exists(result.Glb) || !File.ReadAllBytes(result.Glb).SequenceEqual(bytes))
        {
            var partial = result.Glb + ".part";
            File.WriteAllBytes(partial, bytes);
            if (File.Exists(result.Glb)) File.Delete(result.Glb);
            File.Move(partial, result.Glb);
            result.Written = true;
        }
        return result;
    }

    public static IEnumerable<string> SourcesUnder(string assetsDir)
    {
        var root = Path.GetFullPath(assetsDir);
        return Directory.EnumerateFiles(root, "*", SearchOption.AllDirectories)
            .Where(IsSource)
            .Where(p => !p.Substring(root.Length).Split(Path.DirectorySeparatorChar, '/').Any(part => part.StartsWith(".")))
            .OrderBy(p => p, StringComparer.Ordinal);
    }
}
