import com.anatawa12.gradleRust.CargoToolChain

plugins {
    id("com.github.johnrengelman.shadow") version "6.1.0"
    id("com.anatawa12.gradle-rust")
    java
    application
}

sourceSets.main.get().java.srcDir("src/main/lib")

group = project(":").group
version = project(":").version

repositories {
    mavenCentral()
}

dependencies {
    implementation("com.google.protobuf:protobuf-java:3.6.1")
}

application {
    // use mainClassName for shadow plugin.
    // see https://github.com/johnrengelman/shadow/issues/609
    // see https://github.com/johnrengelman/shadow/pull/612

    //mainClass.set("NativeWrapperRunner")
    @Suppress("DEPRECATION")
    mainClassName = "NativeWrapperRunner"
}

val mainClassName = "com.anatawa12.protobuf.compiler.PluginMain"

// the task to copy jar and write some text for cargo.
// cargo can't refer to files out of cargo project.
val copyJar by tasks.creating(Copy::class) {
    @Suppress("UnstableApiUsage")
    from(tasks.shadowJar.flatMap { it.archiveFile })
    into("native/resources")
    dependsOn(tasks.shadowJar)
    rename { "all.jar" }
    outputs.files("native/resources/main_class_name.txt")
    doLast {
        file("native/resources/main_class_name.txt")
            .writeText(mainClassName)
    }
}

val cargoProj = cargo.projects.create("native") {
    projectDir.set(project.projectDir.resolve("native"))
    destinationDir.set(project.projectDir.resolve("native/target"))
    targetName.set("embedded-jar-launcher")
    toolChain.set(CargoToolChain.cross)
    dependencyTasks.add(copyJar)

    tasks.build.get().dependsOn(buildTask)
}

cargoProj.targets.create("current") {
    dependsOn(copyJar)
    // current architecture should possible to
    // compile with current toolchain
    toolChain.set(CargoToolChain.default)
}

// cross build configuration. this configuration works for my mac environment.
// may not work for your environment
if (project.hasProperty("cross") && project.property("cross") != "false") {
    cargoProj.targets.create("aarch64-unknown-linux-gnu") {
        dependsOn(copyJar)
    }
    cargoProj.targets.create("x86_64-unknown-linux-gnu") {
        dependsOn(copyJar)
    }

    cargoProj.targets.create("aarch64-apple-darwin") {
        // can't build with cross
        toolChain.set(CargoToolChain.default)
        dependsOn(copyJar)
    }
    cargoProj.targets.create("x86_64-apple-darwin") {
        // can't build with cross
        toolChain.set(CargoToolChain.default)
        dependsOn(copyJar)
    }

    cargoProj.targets.create("x86_64-pc-windows-gnu") {
        dependsOn(copyJar)
    }
}

// compiler bug workaround
Unit
