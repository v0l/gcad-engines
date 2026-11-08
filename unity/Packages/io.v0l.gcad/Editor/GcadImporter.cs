using System;
using System.IO;
using UnityEditor;
using UnityEngine;

public static class GcadImporter
{
    const string HelperKey = "io.v0l.gcad.fbx-helper";
    const string AutoKey = "io.v0l.gcad.auto-adapt";

    public static string HelperPath
    {
        get { return EditorPrefs.GetString(HelperKey, Environment.GetEnvironmentVariable("GCAD_FBX_HELPER") ?? ""); }
        set { EditorPrefs.SetString(HelperKey, value ?? ""); }
    }

    public static bool AutoAdapt
    {
        get { return EditorPrefs.GetBool(AutoKey, false); }
        set { EditorPrefs.SetBool(AutoKey, value); }
    }

    public static bool IsGcadPackage(string modelPath)
    {
        if (!modelPath.EndsWith(".fbx", StringComparison.OrdinalIgnoreCase)) return false;
        var directory = Path.GetDirectoryName(modelPath);
        foreach (var receipt in new[] { "gcad-consumer-profile.json", "gcad-root-motion.json", "gcad-shaders.json" })
            if (File.Exists(Path.Combine(directory, receipt))) return true;
        return Directory.Exists(Path.Combine(directory, "textures")) && Path.GetFileName(modelPath) == "model.fbx";
    }

    public static string OutputFor(string modelPath)
    {
        var directory = Path.GetDirectoryName(modelPath).Replace('\\', '/');
        var name = Path.GetFileName(directory);
        return Path.GetDirectoryName(directory).Replace('\\', '/') + "/" + name + "-gcad";
    }

    public static GameObject Adapt(string modelPath)
    {
        var helper = HelperPath;
        if (string.IsNullOrEmpty(helper) || !Path.IsPathRooted(helper) || !File.Exists(helper))
            throw new Exception("Set the absolute path of gcad_fbx_helper in Project Settings > gcad (or GCAD_FBX_HELPER)");
        var output = OutputFor(modelPath);
        if (AssetDatabase.IsValidFolder(output)) AssetDatabase.DeleteAsset(output);
        var importer = (ModelImporter)AssetImporter.GetAtPath(modelPath);
        if (GcadConsumerAdapter.Read(modelPath) != null && !importer.preserveHierarchy)
        {
            importer.preserveHierarchy = true;
            importer.SaveAndReimport();
        }
        return GcadMaterialAdapter.Create(helper, modelPath, output);
    }

    [MenuItem("Assets/gcad/Build Adapted Prefab", true)]
    static bool CanAdaptSelection()
    {
        var path = AssetDatabase.GetAssetPath(Selection.activeObject);
        return !string.IsNullOrEmpty(path) && IsGcadPackage(path);
    }

    [MenuItem("Assets/gcad/Build Adapted Prefab")]
    static void AdaptSelection()
    {
        var path = AssetDatabase.GetAssetPath(Selection.activeObject);
        try
        {
            var prefab = Adapt(path);
            Selection.activeObject = prefab;
            Debug.Log("gcad: adapted " + path + " into " + OutputFor(path));
        }
        catch (Exception e)
        {
            Debug.LogError("gcad: " + path + ": " + e.Message);
        }
    }

    [SettingsProvider]
    static SettingsProvider Settings()
    {
        return new SettingsProvider("Project/gcad", SettingsScope.Project)
        {
            guiHandler = _ =>
            {
                HelperPath = EditorGUILayout.TextField("FBX helper (absolute)", HelperPath);
                AutoAdapt = EditorGUILayout.Toggle("Adapt gcad packages on import", AutoAdapt);
                EditorGUILayout.HelpBox("A gcad package is a folder written by `gcad asset-fbx-experimental`: model.fbx, textures/ and gcad-*.json receipts. Adapting builds a prefab with gcad materials, sockets, colliders, LOD groups, root motion and UV scroll in a sibling <folder>-gcad.", MessageType.Info);
            }
        };
    }
}

sealed class GcadPackagePostprocessor : AssetPostprocessor
{
    static void OnPostprocessAllAssets(string[] imported, string[] deleted, string[] moved, string[] movedFrom)
    {
        if (!GcadImporter.AutoAdapt) return;
        foreach (var path in imported)
        {
            if (!GcadImporter.IsGcadPackage(path)) continue;
            var target = path;
            EditorApplication.delayCall += () =>
            {
                try { GcadImporter.Adapt(target); }
                catch (Exception e) { Debug.LogError("gcad: " + target + ": " + e.Message); }
            };
        }
    }
}
