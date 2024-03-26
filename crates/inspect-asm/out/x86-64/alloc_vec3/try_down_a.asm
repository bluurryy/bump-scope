inspect_asm::alloc_vec3::try_down_a:
	push rbx
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	add rax, -12
	cmp rax, qword ptr [rcx + 8]
	jb .LBB_3
	mov qword ptr [rcx], rax
.LBB_2:
	mov ecx, dword ptr [rsi + 8]
	mov dword ptr [rax + 8], ecx
	mov rcx, qword ptr [rsi]
	mov qword ptr [rax], rcx
	pop rbx
	ret
.LBB_3:
	mov rbx, rsi
	call bump_scope::bump_scope::BumpScope<A,_,_>::do_alloc_sized_in_another_chunk
	mov rsi, rbx
	test rax, rax
	jne .LBB_2
	xor eax, eax
	pop rbx
	ret