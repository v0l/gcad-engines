using System;
using System.IO;
using System.Linq;
using UnityEditor;
using UnityEngine;

public static class GcadSourceSync
{
    const string ToolKey = "io.v0l.gcad.gcad2glb";

    public static string ToolPath
    {
        get { return EditorPrefs.GetString(ToolKey, Environment.GetEnvironmentVariable("GCAD2GLB") ?? "gcad2glb"); }
        set { EditorPrefs.SetString(ToolKey, value ?? ""); }
    }

    static string ProjectRoot
    {
        get { return Path.GetDirectoryName(Application.dataPath); }
    }

    public static string FlagsFor(string assetPath)
    {
        var importer = AssetImporter.GetAtPath(assetPath);
        return importer == null ? "" : importer.userData;
    }

    public static int SyncAll()
    {
        var root = ProjectRoot;
        var cacheDir = GcadCache.CacheDir(root);
        var written = 0;
        foreach (var full in GcadCache.SourcesUnder(Application.dataPath))
        {
            var asset = "Assets" + full.Substring(Application.dataPath.Length).Replace('\\', '/');
            var result = GcadCache.Sync(ToolPath, cacheDir, full, GcadCache.Flags(FlagsFor(asset)));
            if (result.Error != null)
            {
                Debug.LogError("gcad: " + asset + ": " + result.Error);
                continue;
            }
            if (result.Written)
            {
                written++;
                Debug.Log("gcad: " + asset + (result.Cached ? " from Library/gcad-cache " : " built ") + result.Key);
            }
        }
        if (written > 0) AssetDatabase.Refresh();
        return written;
    }

    [InitializeOnLoadMethod]
    static void OnLoad()
    {
        EditorApplication.delayCall += () => SyncAll();
    }

    [MenuItem("Assets/gcad/Rebuild gcad Models")]
    static void Rebuild()
    {
        Debug.Log("gcad: " + SyncAll() + " model(s) updated");
    }
}

sealed class GcadSourcePostprocessor : AssetPostprocessor
{
    static void OnPostprocessAllAssets(string[] imported, string[] deleted, string[] moved, string[] movedFrom)
    {
        if (imported.Concat(moved).Any(GcadCache.IsSource))
            EditorApplication.delayCall += () => GcadSourceSync.SyncAll();
    }
}
