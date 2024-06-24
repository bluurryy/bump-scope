inspect_asm::alloc_zst::try_bumpalo:
	mov rcx, qword ptr [rdi + 16]
	mov rax, qword ptr [rcx + 32]
	cmp rax, qword ptr [rcx]
	jb .LBB0_0
	ret
.LBB0_0:
	mov esi, 1
	xor edx, edx
	jmp qword ptr [rip + bumpalo::Bump::alloc_layout_slow@GOTPCREL]
