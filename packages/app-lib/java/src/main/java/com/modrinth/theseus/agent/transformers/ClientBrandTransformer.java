package com.modrinth.theseus.agent.transformers;

import org.objectweb.asm.Opcodes;
import org.objectweb.asm.tree.ClassNode;
import org.objectweb.asm.tree.InsnNode;
import org.objectweb.asm.tree.LdcInsnNode;
import org.objectweb.asm.tree.MethodNode;

public final class ClientBrandTransformer extends ClassNodeTransformer {
    private static final String BRAND = System.getProperty("modrinth.internal.clientBrand");

    @Override
    protected boolean transform(ClassNode classNode) {
        if (BRAND == null) {
            return false;
        }

        boolean changed = false;
        for (final MethodNode method : classNode.methods) {
            if ("()Ljava/lang/String;".equals(method.desc)
                    && ("getClientModName".equals(method.name) || "getClientBrand".equals(method.name))) {
                method.instructions.clear();
                method.instructions.add(new LdcInsnNode(BRAND));
                method.instructions.add(new InsnNode(Opcodes.ARETURN));
                method.tryCatchBlocks.clear();
                changed = true;
            }
        }
        return changed;
    }
}
