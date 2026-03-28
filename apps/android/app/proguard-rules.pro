# Retrofit
-keepattributes Signature
-keepattributes Exceptions
-keepclassmembers,allowshrinking,allowobfuscation interface * {
    @retrofit2.http.* <methods>;
}
-dontwarn javax.annotation.**
-dontwarn kotlin.Unit
-dontwarn retrofit2.KotlinExtensions
-dontwarn retrofit2.KotlinExtensions$*

# Gson — keep ALL reflection types
-keepattributes Signature
-keepattributes *Annotation*
-keepattributes EnclosingMethod
-keepattributes InnerClasses
-keep class com.google.gson.** { *; }
-keep class com.google.gson.reflect.TypeToken { *; }
-keep class * extends com.google.gson.reflect.TypeToken { *; }
-keep class com.escudo.vpn.data.model.** { *; }
-keep class com.escudo.vpn.data.api.** { *; }

# Retrofit — keep all type information
-keep,allowobfuscation,allowshrinking class retrofit2.Response
-keep class retrofit2.** { *; }
-keepclassmembers,allowshrinking,allowobfuscation interface * {
    @retrofit2.http.* <methods>;
}

# Java reflection — CRITICAL for Gson/Retrofit
-keep class java.lang.reflect.** { *; }
-keep class sun.misc.Unsafe { *; }
-dontwarn sun.misc.Unsafe

# Keep all generic type info needed by Gson
-keep class * implements java.lang.reflect.ParameterizedType { *; }
-keep class * implements java.lang.reflect.Type { *; }

# Kotlin metadata for Gson
-keep class kotlin.Metadata { *; }
-keepclassmembers class * {
    @com.google.gson.annotations.SerializedName <fields>;
}

# WireGuard
-keep class com.wireguard.** { *; }

# Tink / EncryptedSharedPreferences
-dontwarn com.google.errorprone.annotations.**
-dontwarn javax.annotation.**
-dontwarn com.google.api.**
-dontwarn com.google.crypto.tink.**
-keep class com.google.crypto.tink.** { *; }

# Hilt
-keep class dagger.hilt.** { *; }
-keep class javax.inject.** { *; }
-keep class * extends dagger.hilt.android.internal.managers.ViewComponentManager$FragmentContextWrapper { *; }
