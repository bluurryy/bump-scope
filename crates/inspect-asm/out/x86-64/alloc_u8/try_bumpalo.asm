inspect_asm::alloc_u8::try_bumpalo:
	push rbx
	mov rcx, qword ptr [rdi + 16]
	mov rax, qword ptr [rcx + 32]
	dec rax
	cmp rax, qword ptr [rcx]
	jb .LBB_3
	mov qword ptr [rcx + 32], rax
.LBB_2:
	mov byte ptr [rax], sil
	pop rbx
	ret
.LBB_3:
	mov ebx, esi
	mov esi, 1
	mov edx, 1
	call qword ptr [rip + bumpalo::Bump::alloc_layout_slow@GOTPCREL]
	mov esi, ebx
	test rax, rax
	jne .LBB_2
	xor eax, eax
	pop rbx
	ret