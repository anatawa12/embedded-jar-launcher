package com.anatawa12.protobuf.compiler;

import com.google.protobuf.compiler.PluginProtos;

public class PluginMain {
    public static void main(String[] args) {
        System.out.println("hello world!");
        System.out.println("Accessible to protobuf with shadow plugin: "
                + PluginProtos.CodeGeneratorRequest.class);
    }
}
